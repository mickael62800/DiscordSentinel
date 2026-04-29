//! Implementation gRPC du `AutomodService` (Phase 7A).
//! Wrappe `AnalyzeMessageUseCase`. Hot path le plus chaud : un appel par
//! message Discord recu sur les serveurs.

use std::sync::Arc;

use tonic::Request;
use tonic::Response;
use tonic::Status;
use sentinel_proto::automod::v1 as proto;
use sentinel_proto::automod::v1::automod_service_server::AutomodService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::ai::message_analysis::MessageAnalysis;
use crate::domain::enums::moderation::action::Action;
use crate::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::ports::inbound::ai::analyze_message::AnalyzeMessageCommand;
use crate::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use crate::ports::inbound::ai::analyze_message::ContextMessageEntry;
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

        // Validation inputs obligatoires.
        if req.guild_id.is_empty() || req.guild_id.len() > 20 {
            return Err(Status::invalid_argument("guild_id invalide"));
        }
        if req.user_id.is_empty() || req.user_id.len() > 20 {
            return Err(Status::invalid_argument("user_id invalide"));
        }
        if req.content.is_empty() {
            return Err(Status::invalid_argument("content ne peut pas etre vide"));
        }

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
                guild_id: req.guild_id.into(),
                channel_id: req.channel_id.into(),
                user_id: req.user_id.into(),
                username: req.username,
                content: req.content,
                flags,
                message_id: req.message_id.into(),
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
#[path = "tests/automod.rs"]
mod tests;
