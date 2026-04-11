//! Implementation gRPC du `AutomodService` (Phase 7A).
//! Wrappe `AnalyzeMessageUseCase`. Hot path le plus chaud : un appel par
//! message Discord recu sur les serveurs.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use sentinel_proto::automod::v1 as proto;
use sentinel_proto::automod::v1::automod_service_server::AutomodService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::MessageAnalysis;
use crate::domain::value_objects::{Action, DetectionFlags};
use crate::ports::inbound::{AnalyzeMessageCommand, AnalyzeMessageUseCase, ContextMessageEntry};

pub struct AutomodGrpc {
    pub uc: Arc<dyn AnalyzeMessageUseCase>,
}

#[tonic::async_trait]
impl AutomodService for AutomodGrpc {
    async fn analyze_message(
        &self,
        request: Request<proto::AnalyzeMessageRequest>,
    ) -> Result<Response<proto::AnalyzeMessageResponse>, Status> {
        let req = request.into_inner();
        let flags = req
            .flags
            .map(proto_to_flags)
            .unwrap_or(DetectionFlags { spam: false, insult: false, link: false, phishing: false });
        let context_messages = req
            .context_messages
            .into_iter()
            .map(|m| ContextMessageEntry {
                username: m.username,
                content: m.content,
            })
            .collect();
        let analysis = self
            .uc
            .analyze(AnalyzeMessageCommand {
                guild_id: req.guild_id,
                channel_id: req.channel_id,
                user_id: req.user_id,
                username: req.username,
                content: req.content,
                flags,
                message_id: req.message_id,
                timestamp: req.timestamp,
                context_messages,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(analysis_to_proto(analysis)))
    }
}

fn proto_to_flags(p: proto::DetectionFlags) -> DetectionFlags {
    DetectionFlags {
        spam: p.spam,
        insult: p.insult,
        link: p.link,
        phishing: p.phishing,
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

fn analysis_to_proto(a: MessageAnalysis) -> proto::AnalyzeMessageResponse {
    proto::AnalyzeMessageResponse {
        action: action_to_proto(a.action),
        reason: a.reason,
        score: a.score,
        duration: a.duration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proto_to_flags_round_trip_all_true() {
        let p = proto::DetectionFlags { spam: true, insult: true, link: true, phishing: true };
        let f = proto_to_flags(p);
        assert!(f.spam && f.insult && f.link && f.phishing);
    }

    #[test]
    fn proto_to_flags_round_trip_mixed() {
        let p = proto::DetectionFlags { spam: true, insult: false, link: true, phishing: false };
        let f = proto_to_flags(p);
        assert!(f.spam);
        assert!(!f.insult);
        assert!(f.link);
        assert!(!f.phishing);
    }

    #[test]
    fn action_to_proto_all_variants() {
        assert_eq!(action_to_proto(Action::None), proto::Action::None as i32);
        assert_eq!(action_to_proto(Action::Warn), proto::Action::Warn as i32);
        assert_eq!(action_to_proto(Action::Delete), proto::Action::Delete as i32);
        assert_eq!(action_to_proto(Action::Mute), proto::Action::Mute as i32);
        assert_eq!(action_to_proto(Action::Ban), proto::Action::Ban as i32);
    }

    #[test]
    fn analysis_to_proto_full_mapping() {
        let a = MessageAnalysis {
            action: Action::Warn,
            reason: "spam".into(),
            score: 0.65,
            duration: Some(300),
        };
        let p = analysis_to_proto(a);
        assert_eq!(p.action, proto::Action::Warn as i32);
        assert_eq!(p.reason, "spam");
        assert!((p.score - 0.65).abs() < 1e-6);
        assert_eq!(p.duration, Some(300));
    }

    #[test]
    fn analysis_to_proto_no_action() {
        let a = MessageAnalysis {
            action: Action::None,
            reason: String::new(),
            score: 0.0,
            duration: None,
        };
        let p = analysis_to_proto(a);
        assert_eq!(p.action, proto::Action::None as i32);
        assert!(p.duration.is_none());
    }
}
