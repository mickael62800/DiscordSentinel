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

use tonic::{Request, Response, Status};

use sentinel_proto::moderation::v1 as proto;
use sentinel_proto::moderation::v1::moderation_service_server::ModerationService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::{ModerationAction, UserModerationHistory};
use crate::ports::inbound::{LogModerationCommand, ManageModerationUseCase};

pub struct ModerationGrpc {
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
}

#[tonic::async_trait]
impl ModerationService for ModerationGrpc {
    async fn log_action(
        &self,
        request: Request<proto::LogActionRequest>,
    ) -> Result<Response<proto::ModerationAction>, Status> {
        let req = request.into_inner();
        let action = self
            .moderation_uc
            .log_action(LogModerationCommand {
                guild_id: req.guild_id,
                channel_id: req.channel_id,
                moderator_id: req.moderator_id,
                moderator_name: req.moderator_name,
                target_id: req.target_id,
                target_name: req.target_name,
                action_type: req.action_type,
                reason: req.reason,
                gravity: req.gravity,
                duration: req.duration,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(moderation_action_to_proto(action)))
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
        guild_id: a.guild_id,
        channel_id: a.channel_id,
        moderator_id: a.moderator_id,
        moderator_name: a.moderator_name,
        target_id: a.target_id,
        target_name: a.target_name,
        action_type: a.action_type,
        reason: a.reason,
        gravity: a.gravity.map(|g| g.as_str().to_string()),
        duration: a.duration,
        created_at: a.created_at.to_rfc3339(),
    }
}

fn user_history_to_proto(h: UserModerationHistory) -> proto::UserHistory {
    proto::UserHistory {
        target_id: h.target_id,
        target_name: h.target_name,
        total_warns: h.total_warns,
        total_mutes: h.total_mutes,
        total_bans: h.total_bans,
        actions: h.actions.into_iter().map(moderation_action_to_proto).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;
    use crate::domain::value_objects::ModerationGravity;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    fn sample_action() -> ModerationAction {
        ModerationAction {
            id: Uuid::nil(),
            guild_id: "g".into(),
            channel_id: "c".into(),
            moderator_id: "mod".into(),
            moderator_name: "Mod".into(),
            target_id: "u".into(),
            target_name: "Joe".into(),
            action_type: "warn".into(),
            reason: "spam".into(),
            gravity: Some(ModerationGravity::High),
            duration: Some(3600),
            created_at: ts(),
        }
    }

    #[test]
    fn moderation_action_to_proto_full_mapping() {
        let p = moderation_action_to_proto(sample_action());
        assert_eq!(p.guild_id, "g");
        assert_eq!(p.moderator_name, "Mod");
        assert_eq!(p.action_type, "warn");
        assert_eq!(p.reason, "spam");
        assert_eq!(p.gravity.as_deref(), Some("high"));
        assert_eq!(p.duration, Some(3600));
        assert_eq!(p.created_at, ts().to_rfc3339());
    }

    #[test]
    fn moderation_action_to_proto_no_gravity_no_duration() {
        let mut a = sample_action();
        a.gravity = None;
        a.duration = None;
        let p = moderation_action_to_proto(a);
        assert!(p.gravity.is_none());
        assert!(p.duration.is_none());
    }

    #[test]
    fn moderation_action_gravity_low_serialised() {
        let mut a = sample_action();
        a.gravity = Some(ModerationGravity::Low);
        let p = moderation_action_to_proto(a);
        assert_eq!(p.gravity.as_deref(), Some("low"));
    }

    #[test]
    fn user_history_to_proto_full_mapping() {
        let h = UserModerationHistory {
            target_id: "u".into(),
            target_name: "Joe".into(),
            total_warns: 3,
            total_mutes: 1,
            total_bans: 0,
            actions: vec![sample_action(), sample_action()],
        };
        let p = user_history_to_proto(h);
        assert_eq!(p.target_id, "u");
        assert_eq!(p.total_warns, 3);
        assert_eq!(p.total_mutes, 1);
        assert_eq!(p.total_bans, 0);
        assert_eq!(p.actions.len(), 2);
    }

    #[test]
    fn user_history_to_proto_empty_history() {
        let h = UserModerationHistory {
            target_id: "u".into(),
            target_name: "Clean".into(),
            total_warns: 0,
            total_mutes: 0,
            total_bans: 0,
            actions: vec![],
        };
        let p = user_history_to_proto(h);
        assert!(p.actions.is_empty());
    }
}
