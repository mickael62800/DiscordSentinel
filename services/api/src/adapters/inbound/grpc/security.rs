//! Implementation gRPC du `SecurityService` (Phase 7A).
//! Wrappe `ManageSecurityUseCase`.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use sentinel_proto::security::v1 as proto;
use sentinel_proto::security::v1::security_service_server::SecurityService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::SecurityEvent;
use crate::domain::services::security_analyzer::JoinInfo;
use crate::ports::inbound::{
    AnalyzeNewMemberCommand, ManageSecurityUseCase, ReportSecurityEventCommand,
};

pub struct SecurityGrpc {
    pub uc: Arc<dyn ManageSecurityUseCase>,
}

#[tonic::async_trait]
impl SecurityService for SecurityGrpc {
    async fn report_event(
        &self,
        request: Request<proto::ReportEventRequest>,
    ) -> Result<Response<proto::SecurityEvent>, Status> {
        let req = request.into_inner();
        let event = self
            .uc
            .report_event(ReportSecurityEventCommand {
                guild_id: req.guild_id,
                event_type: req.event_type,
                severity: req.severity,
                description: req.description,
                user_ids: req.user_ids,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(security_event_to_proto(event)))
    }

    async fn list_events(
        &self,
        request: Request<proto::ListEventsRequest>,
    ) -> Result<Response<proto::SecurityEventList>, Status> {
        let req = request.into_inner();
        let events = self
            .uc
            .list_events(req.guild_id.as_deref())
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SecurityEventList {
            events: events.into_iter().map(security_event_to_proto).collect(),
        }))
    }

    async fn analyze_new_member(
        &self,
        request: Request<proto::AnalyzeNewMemberRequest>,
    ) -> Result<Response<proto::SecurityDecision>, Status> {
        let req = request.into_inner();
        let recent_joins = req
            .recent_joins
            .into_iter()
            .map(|j| JoinInfo {
                username: j.username,
                has_avatar: j.has_avatar,
                account_created_timestamp: j.account_created_timestamp,
            })
            .collect();
        let decision = self
            .uc
            .analyze_new_member(AnalyzeNewMemberCommand {
                guild_id: req.guild_id,
                user_id: req.user_id,
                username: req.username,
                has_avatar: req.has_avatar,
                account_created_timestamp: req.account_created_timestamp,
                is_bot: req.is_bot,
                recent_joins,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SecurityDecision {
            is_raid: decision.is_raid,
            raid_score: decision.raid_score,
            is_suspicious_account: decision.is_suspicious_account,
            is_alt_account: decision.is_alt_account,
            alt_similar_to: decision.alt_similar_to,
            quarantine: decision.quarantine,
            send_captcha: decision.send_captcha,
            activate_lockdown: decision.activate_lockdown,
            slowmode_secs: decision.slowmode_secs,
            event_type: decision.event_type,
            event_description: decision.event_description,
        }))
    }
}

fn security_event_to_proto(e: SecurityEvent) -> proto::SecurityEvent {
    proto::SecurityEvent {
        id: e.id.to_string(),
        guild_id: e.guild_id,
        event_type: e.event_type,
        severity: e.severity,
        description: e.description,
        user_ids: e.user_ids,
        created_at: e.created_at.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    #[test]
    fn security_event_to_proto_full_mapping() {
        let e = SecurityEvent {
            id: Uuid::nil(),
            guild_id: "g".into(),
            event_type: "raid".into(),
            severity: "critical".into(),
            description: "Mass join detected".into(),
            user_ids: vec!["u1".into(), "u2".into(), "u3".into()],
            created_at: ts(),
        };
        let p = security_event_to_proto(e);
        assert_eq!(p.id, Uuid::nil().to_string());
        assert_eq!(p.guild_id, "g");
        assert_eq!(p.event_type, "raid");
        assert_eq!(p.severity, "critical");
        assert_eq!(p.description, "Mass join detected");
        assert_eq!(p.user_ids.len(), 3);
        assert_eq!(p.user_ids[1], "u2");
        assert_eq!(p.created_at, ts().to_rfc3339());
    }

    #[test]
    fn security_event_to_proto_no_users() {
        let e = SecurityEvent {
            id: Uuid::nil(),
            guild_id: "g".into(),
            event_type: "scan".into(),
            severity: "info".into(),
            description: String::new(),
            user_ids: vec![],
            created_at: ts(),
        };
        let p = security_event_to_proto(e);
        assert!(p.user_ids.is_empty());
        assert_eq!(p.severity, "info");
    }
}
