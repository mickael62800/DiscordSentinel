//! Implementation gRPC du `SecurityService` (Phase 7A).
//! Wrappe `ManageSecurityUseCase`.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use sentinel_proto::security::v1 as proto;
use sentinel_proto::security::v1::security_service_server::SecurityService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::SecurityEvent;
use crate::ports::inbound::{ManageSecurityUseCase, ReportSecurityEventCommand};

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
