//! Client API du automod module.
//!
//! Phase 7A -- Migration gRPC complete : `analyze` est le **hot path le plus
//! chaud du projet** (un appel par message Discord recu sur tous les
//! serveurs). Le gain perf gRPC est ici maximal.
//!
//! ## Comportement si l'API tombe
//!
//! Le circuit breaker (5 echecs / 10s) court-circuite immediatement les
//! appels suivants. Pendant l'ouverture, `analyze` retourne `Err("API
//! indisponible")` et le bot **n'applique aucune action de moderation**.
//! Comportement par defaut : laisser passer le message (ne pas faire de
//! faux positifs basees sur une API down). Cote handler, le timeout
//! original de 5s est conserve pour ne pas bloquer le bot.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::SentinelGrpcClient;

use sentinel_proto::automod::v1 as proto;

use super::detectors::DetectionFlags;

#[derive(Debug, Serialize)]
pub struct AnalyzeRequest {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub username: String,
    pub content: String,
    pub flags: DetectionFlags,
    pub metadata: MessageMetadata,
    pub context_messages: Vec<ContextMessage>,
}

#[derive(Debug, Serialize)]
pub struct ContextMessage {
    pub username: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct MessageMetadata {
    pub message_id: MessageId,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AnalyzeResponse {
    pub action: Action,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub score: Option<f64>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    None,
    Warn,
    Delete,
    Mute,
    Ban,
}

pub struct ApiClient {
    #[allow(dead_code)]
    pub base: Arc<BaseApiClient>,
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>, grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { base, grpc }
    }

    /// gRPC `AutomodService.AnalyzeMessage` (hot path le plus chaud).
    pub async fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzeResponse, String> {
        let req = proto::AnalyzeMessageRequest {
            guild_id: request.guild_id.clone(),
            channel_id: request.channel_id.clone(),
            user_id: request.user_id.clone(),
            username: request.username.clone(),
            content: request.content.clone(),
            flags: Some(proto::DetectionFlags {
                spam: request.flags.spam,
                insult: request.flags.insult,
                link: request.flags.link,
                phishing: request.flags.phishing,
            }),
            message_id: request.metadata.message_id.clone(),
            timestamp: request.metadata.timestamp.clone(),
            context_messages: request
                .context_messages
                .iter()
                .map(|m| proto::ContextMessage {
                    username: m.username.clone(),
                    content: m.content.clone(),
                })
                .collect(),
        };
        let mut client = self.grpc.automod();
        let resp = self
            .grpc
            .guarded(|| async move {
                client.analyze_message(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(AnalyzeResponse {
            action: proto_action_to_action(resp.action),
            reason: if resp.reason.is_empty() {
                None
            } else {
                Some(resp.reason)
            },
            duration: resp.duration,
            score: Some(resp.score),
        })
    }

    // analyze_image supprime -- migre vers ai-worker (async queue + Redis).
}

fn proto_action_to_action(value: i32) -> Action {
    match proto::Action::try_from(value).unwrap_or(proto::Action::None) {
        proto::Action::None => Action::None,
        proto::Action::Warn => Action::Warn,
        proto::Action::Delete => Action::Delete,
        proto::Action::Mute => Action::Mute,
        proto::Action::Ban => Action::Ban,
    }
}

use sentinel_shared::grpc_client::grpc_err_to_string;
use sentinel_api::domain::entities::system::discord_ids::MessageId;
use crate::domain::entities::system::discord_ids::ChannelId;
use sentinel_api::domain::entities::system::discord_ids::UserId;
use sentinel_api::domain::entities::system::discord_ids::GuildId;
