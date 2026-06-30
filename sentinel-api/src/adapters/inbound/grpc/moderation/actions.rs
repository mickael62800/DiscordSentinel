//! Implementation gRPC du `ModerationService` (v1).
//!
//! Scope reduit volontairement (cf. moderation.proto) :
//! - `LogAction` : hot path appele a chaque sanction.
//! - `GetHistory` : consultation frequente.
//!
//! Les autres methodes du moderation-bot (evidence/review/modstats/pending)
//! continueront a passer par HTTP tant que le `ManageModerationUseCase`
//! n'expose pas ces operations.

use std::sync::Arc;

use sentinel_proto::moderation::v1 as proto;
use sentinel_proto::moderation::v1::moderation_service_server::ModerationService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::ports::inbound::moderation::manage_moderation::LogModerationCommand;
use crate::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use crate::ports::inbound::moderation::manage_reminders::CreateReminderCommand;
use crate::ports::inbound::moderation::manage_reminders::ManageRemindersUseCase;
use sentinel_core::domain::entities::moderation::action::applied::ModerationAction;
use sentinel_core::domain::entities::moderation::action::applied::UserModerationHistory;
pub struct ModerationGrpc {
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
    /// Auto-creation des rappels/enregistrements d'expiration pour les sanctions
    /// temporaires journalisees par le bot (chemin gRPC). Aligne le comportement
    /// sur le handler HTTP `log_action`.
    pub reminders_uc: Arc<dyn ManageRemindersUseCase>,
}

#[tonic::async_trait]
impl ModerationService for ModerationGrpc {
    async fn log_action(
        &self,
        request: Request<proto::LogActionRequest>,
    ) -> Result<Response<proto::ModerationAction>, Status> {
        let req = request.into_inner();
        let skip_strike = req.skip_strike;
        let action_type = req.action_type.clone();
        let duration = req.duration;
        let cmd = LogModerationCommand {
            guild_id: req.guild_id.into(),
            channel_id: req.channel_id.into(),
            moderator_id: req.moderator_id,
            moderator_name: req.moderator_name,
            target_id: req.target_id,
            target_name: req.target_name,
            action_type: req.action_type,
            reason: req.reason,
            gravity: req.gravity,
            duration: req.duration,
        };

        // skip_strike : sanction d'escalade auto deja adossee a un strike compte.
        // On journalise l'action SANS rejouer le strike (anti double-strike) en
        // passant par `log_action` plutot que `log_action_with_strike`.
        let proto_action = if skip_strike {
            let action = self
                .moderation_uc
                .log_action(cmd)
                .await
                .map_err(domain_to_status)?;
            moderation_action_to_proto(action)
        } else {
            // Phase 7B : orchestration atomique action+strike via le service.
            let logged = self
                .moderation_uc
                .log_action_with_strike(cmd)
                .await
                .map_err(domain_to_status)?;
            let mut pa = moderation_action_to_proto(logged.action);
            if let Some(sr) = logged.strike {
                pa.strikes_count = Some(sr.active_count);
                pa.escalation_action = sr.escalation_action;
                pa.escalation_duration = sr.escalation_duration;
            }
            pa
        };

        // Auto-creation du rappel/enregistrement d'expiration pour TOUTE sanction
        // temporaire (ban_temp / mute_temp), quel que soit le chemin (bot gRPC).
        // C'est ce qui alimente le job worker d'auto-unban a l'expiration.
        if sentinel_core::domain::enums::moderation::moderation_action_type::ModerationActionType::is_temporary_str(&action_type) {
            if let Some(dur) = duration {
                let action_uuid = proto_action.id.parse().unwrap_or_else(|_| uuid::Uuid::nil());
                if let Err(e) = self
                    .reminders_uc
                    .create_reminder(CreateReminderCommand {
                        guild_id: proto_action.guild_id.clone().into(),
                        moderator_id: proto_action.moderator_id.clone(),
                        moderator_name: proto_action.moderator_name.clone(),
                        target_id: proto_action.target_id.clone(),
                        target_name: proto_action.target_name.clone(),
                        action_type: action_type.clone(),
                        reason: proto_action.reason.clone(),
                        action_id: action_uuid,
                        duration_secs: dur,
                        // Sur le chemin gRPC on applique le defaut (1h) cote service.
                        remind_before_secs: 0,
                    })
                    .await
                {
                    tracing::error!(
                        error = %e,
                        action_id = %proto_action.id,
                        action_type = %action_type,
                        "INCOHERENCE : sanction temporaire sans reminder (gRPC)"
                    );
                }
            }
        }

        Ok(Response::new(proto_action))
    }

    async fn get_history(
        &self,
        request: Request<proto::GetHistoryRequest>,
    ) -> Result<Response<proto::UserHistory>, Status> {
        let req = request.into_inner();
        let history = self
            .moderation_uc
            .get_history(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(user_history_to_proto(history)))
    }
}

fn moderation_action_to_proto(a: ModerationAction) -> proto::ModerationAction {
    proto::ModerationAction {
        id: a.id.to_string(),
        guild_id: a.guild_id.into(),
        channel_id: a.channel_id.into(),
        moderator_id: a.moderator_id,
        moderator_name: a.moderator_name,
        target_id: a.target_id,
        target_name: a.target_name,
        action_type: a.action_type,
        reason: a.reason,
        gravity: a.gravity.map(|g| g.as_str().to_string()),
        duration: a.duration,
        created_at: a.created_at.to_rfc3339(),
        // Renseignes uniquement en reponse de LogAction (overrides plus bas).
        strikes_count: None,
        escalation_action: None,
        escalation_duration: None,
    }
}

fn user_history_to_proto(h: UserModerationHistory) -> proto::UserHistory {
    proto::UserHistory {
        target_id: h.target_id,
        target_name: h.target_name,
        total_warns: h.total_warns,
        total_mutes: h.total_mutes,
        total_bans: h.total_bans,
        actions: h
            .actions
            .into_iter()
            .map(moderation_action_to_proto)
            .collect(),
    }
}

#[cfg(test)]
#[path = "tests/actions.rs"]
mod tests;
