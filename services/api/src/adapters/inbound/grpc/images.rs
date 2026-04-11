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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_to_proto_all_variants() {
        assert_eq!(action_to_proto(Action::None), proto::Action::None as i32);
        assert_eq!(action_to_proto(Action::Warn), proto::Action::Warn as i32);
        assert_eq!(action_to_proto(Action::Delete), proto::Action::Delete as i32);
        assert_eq!(action_to_proto(Action::Mute), proto::Action::Mute as i32);
        assert_eq!(action_to_proto(Action::Ban), proto::Action::Ban as i32);
    }

    #[test]
    fn classification_to_proto_mapping() {
        let c = ImageClassification { label: "weapon".into(), confidence: 0.92 };
        let p = classification_to_proto(c);
        assert_eq!(p.label, "weapon");
        assert!((p.confidence - 0.92).abs() < 1e-6);
    }

    #[test]
    fn analysis_to_proto_full_mapping() {
        let a = ImageAnalysis {
            action: Action::Delete,
            reason: "violence detectee".into(),
            score: 0.87,
            duration: Some(150),
            classifications: vec![
                ImageClassification { label: "violence".into(), confidence: 0.87 },
                ImageClassification { label: "neutral".into(), confidence: 0.13 },
            ],
        };
        let p = analysis_to_proto(a);
        assert_eq!(p.action, proto::Action::Delete as i32);
        assert_eq!(p.reason, "violence detectee");
        assert!((p.score - 0.87).abs() < 1e-6);
        assert_eq!(p.duration, Some(150));
        assert_eq!(p.classifications.len(), 2);
        assert_eq!(p.classifications[0].label, "violence");
    }

    #[test]
    fn analysis_to_proto_no_action_no_classifications() {
        let a = ImageAnalysis {
            action: Action::None,
            reason: "ok".into(),
            score: 0.0,
            duration: None,
            classifications: vec![],
        };
        let p = analysis_to_proto(a);
        assert_eq!(p.action, proto::Action::None as i32);
        assert!(p.classifications.is_empty());
        assert!(p.duration.is_none());
    }
}
