//! Implementation gRPC du `ImagesService` (Phase 7A).
//! Wrappe `AnalyzeImageUseCase`. Avantage majeur : payload binaire natif
//! (pas de base64), gain ~33% sur la bande passante vs l'ancien HTTP+JSON.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use sentinel_proto::images::v1 as proto;
use sentinel_proto::images::v1::images_service_server::ImagesService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::{ImageAnalysis, ImageClassification};
use crate::domain::value_objects::Action;
use crate::ports::inbound::{AnalyzeImageCommand, AnalyzeImageUseCase};

pub struct ImagesGrpc {
    pub uc: Arc<dyn AnalyzeImageUseCase>,
}

#[tonic::async_trait]
impl ImagesService for ImagesGrpc {
    async fn analyze_image(
        &self,
        request: Request<proto::AnalyzeImageRequest>,
    ) -> Result<Response<proto::AnalyzeImageResponse>, Status> {
        let req = request.into_inner();
        let analysis = self
            .uc
            .analyze_image(AnalyzeImageCommand {
                guild_id: req.guild_id,
                channel_id: req.channel_id,
                user_id: req.user_id,
                username: req.username,
                message_id: req.message_id,
                image_bytes: req.image_data,
                content_type: req.content_type,
                filename: req.filename,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(analysis_to_proto(analysis)))
    }
}

fn action_to_proto(a: Action) -> i32 {
    match a {
        Action::None => proto::Action::None as i32,
        Action::Warn => proto::Action::Warn as i32,
        Action::Delete => proto::Action::Delete as i32,
        Action::Mute => proto::Action::Mute as i32,
        Action::Ban => proto::Action::Ban as i32,
    }
}

fn classification_to_proto(c: ImageClassification) -> proto::Classification {
    proto::Classification {
        label: c.label,
        confidence: c.confidence,
    }
}

fn analysis_to_proto(a: ImageAnalysis) -> proto::AnalyzeImageResponse {
    proto::AnalyzeImageResponse {
        action: action_to_proto(a.action),
        reason: a.reason,
        score: a.score,
        duration: a.duration,
        classifications: a.classifications.into_iter().map(classification_to_proto).collect(),
    }
}
