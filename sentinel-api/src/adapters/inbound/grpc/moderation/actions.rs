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
use crate::ports::inbound::moderation::moderation_copilot::ModerationCopilotUseCase;
use sentinel_core::domain::entities::moderation::action::applied::ModerationAction;
use sentinel_core::domain::entities::moderation::action::applied::UserModerationHistory;
use sentinel_core::domain::entities::moderation::copilot::MemberModerationContext;
use sentinel_core::domain::entities::moderation::copilot::PrecedentDistribution;
use sentinel_core::domain::entities::moderation::copilot::SanctionSuggestion;
pub struct ModerationGrpc {
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
    /// Auto-creation des rappels/enregistrements d'expiration pour les sanctions
    /// temporaires journalisees par le bot (chemin gRPC). Aligne le comportement
    /// sur le handler HTTP `log_action`.
    pub reminders_uc: Arc<dyn ManageRemindersUseCase>,
    /// Copilote de moderation (lecture seule, consultatif). Chemin bot/interne
    /// de confiance : aucune autorisation ici (le bot verifie la permission de
    /// moderation Discord avant d'appeler), coherent avec `log_action`/`get_history`.
    pub moderation_copilot_uc: Arc<dyn ModerationCopilotUseCase>,
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

        // BUG #1 : un unban (quel que soit le chemin — bot `/unban`, client, HTTP)
        // doit annuler les rappels d'auto-unban encore actifs pour cet utilisateur,
        // sinon le worker leverait un ban plus recent applique entre-temps. On
        // centralise ici pour couvrir le bot `/unban` qui journalise directement.
        // Best-effort : un echec ne bloque pas l'action (deja appliquee cote Discord).
        if action_type == "unban" {
            match self
                .reminders_uc
                .cancel_for_target(
                    proto_action.guild_id.as_str(),
                    proto_action.target_id.as_str(),
                )
                .await
            {
                Ok(n) if n > 0 => tracing::info!(
                    guild_id = %proto_action.guild_id,
                    target_id = %proto_action.target_id,
                    cancelled = n,
                    "Rappels d'auto-unban annules suite a un unban (gRPC)"
                ),
                Ok(_) => {}
                Err(e) => tracing::warn!(
                    error = %e,
                    guild_id = %proto_action.guild_id,
                    target_id = %proto_action.target_id,
                    "Echec annulation des rappels d'auto-unban lors de l'unban (gRPC)"
                ),
            }
        }

        // BUG #8 : creation du rappel d'expiration UNIQUEMENT pour les bans
        // temporaires. Les mutes temporaires (`mute_temp`, y compris l'escalade)
        // expirent seuls via le timeout Discord : leur creer un rappel produirait
        // un DM "1h avant" qui se declenche pour rien. Seul `ban_temp` alimente le
        // job worker d'auto-unban a l'expiration.
        if action_type == "ban_temp" {
            if let Some(dur) = duration {
                let action_uuid = proto_action
                    .id
                    .parse()
                    .unwrap_or_else(|_| uuid::Uuid::nil());
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

    async fn get_member_context(
        &self,
        request: Request<proto::GetMemberContextRequest>,
    ) -> Result<Response<proto::MemberModerationContext>, Status> {
        let req = request.into_inner();
        let context = self
            .moderation_copilot_uc
            .get_member_context(
                &req.guild_id,
                &req.user_id,
                req.lookback_days,
                req.min_precedents,
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(member_context_to_proto(context)))
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

fn counts_to_proto(counts: Vec<(String, u32)>) -> Vec<proto::ActionCount> {
    counts
        .into_iter()
        .map(|(action, count)| proto::ActionCount { action, count })
        .collect()
}

fn precedents_to_proto(p: PrecedentDistribution) -> proto::PrecedentDistribution {
    proto::PrecedentDistribution {
        flag_category: p.flag_category,
        counts_by_action: counts_to_proto(p.counts_by_action),
        total: p.total,
    }
}

fn suggestion_to_proto(s: SanctionSuggestion) -> proto::SanctionSuggestion {
    proto::SanctionSuggestion {
        action: s.action.map(|a| a.as_str().to_string()),
        basis: s.basis.as_str().to_string(),
        rationale: s.rationale,
        precedent_count: s.precedent_count,
    }
}

fn member_context_to_proto(c: MemberModerationContext) -> proto::MemberModerationContext {
    proto::MemberModerationContext {
        active_strikes: c.active_strikes,
        sanctions_by_type: counts_to_proto(c.sanctions_by_type),
        last_sanction_at: c.last_sanction_at.map(|d| d.to_rfc3339()),
        open_reviews: c.open_reviews,
        precedents: Some(precedents_to_proto(c.precedents)),
        suggestion: Some(suggestion_to_proto(c.suggestion)),
    }
}

#[cfg(test)]
#[path = "tests/actions.rs"]
mod tests;
