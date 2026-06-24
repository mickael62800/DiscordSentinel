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
use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use sentinel_core::domain::entities::ai::message_analysis::MessageAnalysis;
use sentinel_core::domain::enums::moderation::action::Action;
use sentinel_core::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::ports::inbound::ai::analyze_message::AnalyzeMessageCommand;
use crate::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use crate::ports::inbound::ai::analyze_message::ContextMessageEntry;
pub struct AutomodGrpc {
    pub uc: Arc<dyn AnalyzeMessageUseCase>,
    pub broadcaster: Arc<EventBroadcaster>,
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

        // Capture pour le broadcast WS (le live tail web de l'historique
        // d'analyse) avant que `req` ne soit consomme dans la commande.
        let guild_id_evt = req.guild_id.clone();
        let username_evt = req.username.clone();

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

        // Metrique : compte les decisions de routage automod (observabilite —
        // taux carte/auto/rien, sevères, suppressions de lien). Scrapé via /metrics.
        use sentinel_core::domain::services::moderation::automod_routing::Routing;
        let route_label = match analysis.route {
            Routing::None => "none",
            Routing::Card => "card",
            Routing::Auto => "auto",
        };
        metrics::counter!(
            "automod_decisions_total",
            "route" => route_label,
            "severe" => if analysis.severe { "true" } else { "false" },
            "link_delete" => if analysis.auto_delete_link { "true" } else { "false" },
        )
        .increment(1);

        // Push WS : previent le dashboard (historique d'analyse) en temps reel
        // quand une action est prise, au lieu d'un polling cote web.
        if analysis.action != Action::None {
            self.broadcaster.broadcast(
                "infraction_new",
                serde_json::json!({
                    "guild_id": guild_id_evt,
                    "username": username_evt,
                    "action": analysis.action.as_str(),
                    "reason": &analysis.reason,
                }),
            );
        }

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

fn routing_to_proto(r: sentinel_core::domain::services::moderation::automod_routing::Routing) -> i32 {
    use sentinel_core::domain::services::moderation::automod_routing::Routing;
    match r {
        Routing::None => proto::Routing::None as i32,
        Routing::Card => proto::Routing::Card as i32,
        Routing::Auto => proto::Routing::Auto as i32,
    }
}

fn analysis_to_proto(a: MessageAnalysis) -> proto::AnalyzeMessageResponse {
    proto::AnalyzeMessageResponse {
        action: action_to_proto(a.action),
        reason: a.reason,
        score: a.score,
        duration: a.duration,
        route: routing_to_proto(a.route),
        severe: a.severe,
        auto_delete_link: a.auto_delete_link,
    }
}


#[cfg(test)]
#[path = "tests/automod.rs"]
mod tests;
