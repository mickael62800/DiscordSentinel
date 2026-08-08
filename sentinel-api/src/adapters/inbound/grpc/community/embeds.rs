//! Implementation gRPC du `EmbedsService`.
//!
//! Wrappe la partie « report » de `ManageEmbedsUseCase`. Remplace l'endpoint
//! HTTP `POST /api/embeds/by-id/{id}/posted` appele par le bot apres publication.

use std::sync::Arc;

use sentinel_proto::embeds::v1 as proto;
use sentinel_proto::embeds::v1::embeds_service_server::EmbedsService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use sentinel_core::ports::inbound::community::manage_embeds::ManageEmbedsUseCase;

pub struct EmbedsGrpc {
    pub uc: Arc<dyn ManageEmbedsUseCase>,
}

#[tonic::async_trait]
impl EmbedsService for EmbedsGrpc {
    async fn record_posted(
        &self,
        request: Request<proto::RecordPostedRequest>,
    ) -> Result<Response<proto::EmbedsAck>, Status> {
        let req = request.into_inner();
        let id = uuid::Uuid::parse_str(&req.embed_id)
            .map_err(|_| Status::invalid_argument("embed_id invalide (UUID attendu)"))?;
        self.uc
            .record_posted(id, &req.channel_id, &req.message_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::EmbedsAck {}))
    }
}
