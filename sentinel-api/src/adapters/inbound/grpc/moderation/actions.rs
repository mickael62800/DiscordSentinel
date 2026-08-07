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
use sentinel_core::ports::inbound::moderation::assess_target_risk::{
    AssessTargetRiskCommand, AssessTargetRiskUseCase,
};
use sentinel_core::ports::inbound::moderation::manage_notes::{AddNoteCommand, ManageNotesUseCase};
use sentinel_core::ports::inbound::moderation::read_modstats::ReadModstatsUseCase;
use sentinel_core::ports::outbound::moderation::evidence_repository::EvidenceRepository;
use sentinel_core::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use sentinel_core::ports::outbound::moderation::pending_action_repository::PendingActionRepository;
use sentinel_core::ports::outbound::moderation::review_repository::ReviewRepository;
use sentinel_core::ports::inbound::moderation::cancel_action::{
    CancelModerationActionUseCase, CancelOutcome,
};
use sentinel_core::ports::inbound::moderation::manage_moderation::LogModerationCommand;
use sentinel_core::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use sentinel_core::ports::inbound::moderation::manage_reminders::CreateReminderCommand;
use sentinel_core::ports::inbound::moderation::manage_reminders::ManageRemindersUseCase;
use sentinel_core::ports::inbound::moderation::moderation_copilot::ModerationCopilotUseCase;
use sentinel_core::domain::entities::moderation::action::applied::ModerationAction;
use sentinel_core::domain::entities::moderation::action::applied::UserModerationHistory;
use sentinel_core::domain::entities::moderation::copilot::MemberModerationContext;
use sentinel_core::domain::entities::moderation::copilot::PrecedentDistribution;
use sentinel_core::domain::entities::moderation::copilot::SanctionSuggestion;
pub struct ModerationGrpc {
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
    /// Annulation d'une action (/unwarn). Meme use case que le HTTP.
    pub cancel_action_uc: Arc<dyn CancelModerationActionUseCase>,
    /// Auto-creation des rappels/enregistrements d'expiration pour les sanctions
    /// temporaires journalisees par le bot (chemin gRPC). Aligne le comportement
    /// sur le handler HTTP `log_action`.
    pub reminders_uc: Arc<dyn ManageRemindersUseCase>,
    /// Copilote de moderation (lecture seule, consultatif). Chemin bot/interne
    /// de confiance : aucune autorisation ici (le bot verifie la permission de
    /// moderation Discord avant d'appeler), coherent avec `log_action`/`get_history`.
    pub moderation_copilot_uc: Arc<dyn ModerationCopilotUseCase>,

    // ── Ports du dossier de moderation (ex-HTTP) ──
    pub assess_target_risk_uc: Arc<dyn AssessTargetRiskUseCase>,
    pub modstats_uc: Arc<dyn ReadModstatsUseCase>,
    pub notes_uc: Arc<dyn ManageNotesUseCase>,
    pub evidence_repo: Arc<dyn EvidenceRepository>,
    pub review_repo: Arc<dyn ReviewRepository>,
    pub pending_action_repo: Arc<dyn PendingActionRepository>,
    pub infractions_uc: Arc<dyn ManageInfractionsUseCase>,
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

    async fn cancel_action(
        &self,
        request: Request<proto::CancelActionRequest>,
    ) -> Result<Response<proto::CancelActionResponse>, Status> {
        let req = request.into_inner();
        let action_id = req
            .action_id
            .parse()
            .map_err(|_| Status::invalid_argument("action_id doit etre un UUID"))?;
        // Meme use case que le handler HTTP : l'effet Discord inverse et
        // l'annulation du rappel d'auto-unban ne peuvent pas diverger selon
        // le transport.
        let outcome = self
            .cancel_action_uc
            .cancel(action_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CancelActionResponse {
            cancelled: outcome == CancelOutcome::Cancelled,
        }))
    }

    // ── Garde-fous UX ──

    async fn assess_target_risk(
        &self,
        request: Request<proto::AssessTargetRiskRequest>,
    ) -> Result<Response<proto::TargetRiskDecision>, Status> {
        let req = request.into_inner();
        let d = self
            .assess_target_risk_uc
            .assess(AssessTargetRiskCommand {
                guild_id: req.guild_id,
                account_age_days: req.account_age_days,
                is_bot: req.is_bot,
                has_mod_perms: req.has_mod_perms,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::TargetRiskDecision {
            risky: d.risky,
            reason: d.reason,
        }))
    }

    async fn count_moderator_actions(
        &self,
        request: Request<proto::CountModeratorActionsRequest>,
    ) -> Result<Response<proto::CountModeratorActionsResponse>, Status> {
        let req = request.into_inner();
        let count = self
            .moderation_uc
            .count_recent_mod_actions(&req.guild_id, &req.moderator_id, req.window_secs as i64)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CountModeratorActionsResponse {
            count: count.max(0) as u32,
        }))
    }

    // ── Dossier et suivi ──

    async fn list_active_reminders(
        &self,
        request: Request<proto::ListActiveRemindersRequest>,
    ) -> Result<Response<proto::ListActiveRemindersResponse>, Status> {
        let req = request.into_inner();
        let reminders = self
            .reminders_uc
            .list_by_guild(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ListActiveRemindersResponse {
            reminders: reminders
                .into_iter()
                .map(|r| proto::SanctionReminder {
                    moderator_name: r.moderator_name,
                    target_id: r.target_id,
                    action_type: r.action_type,
                    reason: r.reason,
                    expires_at: r.expires_at.to_rfc3339(),
                })
                .collect(),
        }))
    }

    async fn get_mod_stats(
        &self,
        request: Request<proto::GetModStatsRequest>,
    ) -> Result<Response<proto::GetModStatsResponse>, Status> {
        let req = request.into_inner();
        // 0 = « laisse le serveur decider » : evite d'imposer au bot de
        // connaitre la fenetre par defaut.
        let days = if req.days <= 0 { 30 } else { req.days };
        let entries = self
            .modstats_uc
            .modstats(&req.guild_id, days)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::GetModStatsResponse {
            entries: entries
                .into_iter()
                .map(|m| proto::ModStatsEntry {
                    moderator_id: m.moderator_id,
                    moderator_name: m.moderator_name,
                    total: m.total,
                    warns: m.warns,
                    mutes: m.mutes,
                    bans: m.bans,
                    kicks: m.kicks,
                })
                .collect(),
        }))
    }

    async fn add_note(
        &self,
        request: Request<proto::AddNoteRequest>,
    ) -> Result<Response<proto::AddNoteResponse>, Status> {
        let req = request.into_inner();
        let note = self
            .notes_uc
            .add_note(AddNoteCommand {
                guild_id: req.guild_id.into(),
                user_id: req.user_id.into(),
                author_id: req.author_id,
                author_name: req.author_name,
                content: req.content,
                category: req.category,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::AddNoteResponse {
            id: note.id.to_string(),
        }))
    }

    // ── Preuves ──

    async fn add_evidence(
        &self,
        request: Request<proto::AddEvidenceRequest>,
    ) -> Result<Response<proto::EvidenceEntry>, Status> {
        let req = request.into_inner();
        let action_id = parse_uuid_arg(&req.action_id, "action_id")?;
        let e = self
            .evidence_repo
            .add(
                action_id,
                &req.url,
                req.description.as_deref(),
                &req.uploaded_by,
                &req.uploaded_by_name,
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(evidence_to_proto(e)))
    }

    async fn list_evidence(
        &self,
        request: Request<proto::ListEvidenceRequest>,
    ) -> Result<Response<proto::ListEvidenceResponse>, Status> {
        let req = request.into_inner();
        let action_id = parse_uuid_arg(&req.action_id, "action_id")?;
        let entries = self
            .evidence_repo
            .list(action_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ListEvidenceResponse {
            entries: entries.into_iter().map(evidence_to_proto).collect(),
        }))
    }

    // ── File de relecture ──

    async fn add_review(
        &self,
        request: Request<proto::AddReviewRequest>,
    ) -> Result<Response<proto::ReviewEntry>, Status> {
        let req = request.into_inner();
        let action_id = parse_uuid_arg(&req.action_id, "action_id")?;
        let r = self
            .review_repo
            .add(
                action_id,
                &req.guild_id,
                &req.added_by,
                &req.added_by_name,
                req.reason.as_deref(),
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(review_to_proto(r)))
    }

    async fn list_pending_reviews(
        &self,
        request: Request<proto::ListPendingReviewsRequest>,
    ) -> Result<Response<proto::ListPendingReviewsResponse>, Status> {
        let req = request.into_inner();
        let entries = self
            .review_repo
            .list_pending(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ListPendingReviewsResponse {
            entries: entries.into_iter().map(review_to_proto).collect(),
        }))
    }

    async fn resolve_review(
        &self,
        request: Request<proto::ResolveReviewRequest>,
    ) -> Result<Response<proto::ResolveReviewResponse>, Status> {
        let req = request.into_inner();
        let review_id = parse_uuid_arg(&req.review_id, "review_id")?;
        let resolved = self
            .review_repo
            .resolve(
                review_id,
                &req.reviewer_id,
                &req.reviewer_name,
                req.notes.as_deref(),
                &req.status,
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ResolveReviewResponse { resolved }))
    }

    // ── Mode apprenti ──

    async fn resolve_pending_action(
        &self,
        request: Request<proto::ResolvePendingActionRequest>,
    ) -> Result<Response<proto::ResolvePendingActionResponse>, Status> {
        let req = request.into_inner();
        let id = parse_uuid_arg(&req.action_id, "action_id")?;
        self.pending_action_repo
            .resolve(id, &req.status, &req.reviewed_by)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ResolvePendingActionResponse {}))
    }

    async fn count_user_infractions(
        &self,
        request: Request<proto::CountUserInfractionsRequest>,
    ) -> Result<Response<proto::UserInfractionCounts>, Status> {
        let req = request.into_inner();
        if req.guild_id.is_empty() || req.user_id.is_empty() {
            return Err(Status::invalid_argument("guild_id et user_id requis"));
        }
        let c = self
            .infractions_uc
            .count_user_infractions(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::UserInfractionCounts {
            warns: c.warns,
            deletes: c.deletes,
            mutes: c.mutes,
            bans: c.bans,
            total: c.total,
        }))
    }
}

/// Parse un UUID d'argument, en nommant le champ fautif dans l'erreur.
fn parse_uuid_arg(raw: &str, champ: &str) -> Result<uuid::Uuid, Status> {
    raw.parse()
        .map_err(|_| Status::invalid_argument(format!("{champ} doit etre un UUID")))
}

fn evidence_to_proto(
    e: sentinel_core::ports::outbound::moderation::evidence_repository::EvidenceEntry,
) -> proto::EvidenceEntry {
    proto::EvidenceEntry {
        id: e.id.to_string(),
        action_id: e.action_id.to_string(),
        url: e.url,
        description: e.description,
        uploaded_by: e.uploaded_by,
        uploaded_by_name: e.uploaded_by_name,
        uploaded_at: e.uploaded_at.to_rfc3339(),
    }
}

fn review_to_proto(
    r: sentinel_core::ports::outbound::moderation::review_repository::ReviewEntry,
) -> proto::ReviewEntry {
    proto::ReviewEntry {
        id: r.id.to_string(),
        action_id: r.action_id.to_string(),
        guild_id: r.guild_id.into(),
        added_by: r.added_by,
        added_by_name: r.added_by_name,
        reason: r.reason,
        status: r.status,
        reviewer_id: r.reviewer_id,
        reviewer_name: r.reviewer_name,
        reviewer_notes: r.reviewer_notes,
        added_at: r.added_at.to_rfc3339(),
        resolved_at: r.resolved_at.map(|d| d.to_rfc3339()),
        action_type: r.action_type,
        target_name: r.target_name,
        action_reason: r.action_reason,
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
