//! Client API du welcome-bot.
//!
//! Phase 7A — Migration gRPC :
//! - `is_known_member` -> `MembersService.GetMember` (hot path : a chaque
//!   nouveau membre rejoignant un serveur).
//! - `get_config` -> `WelcomeService.GetConfig` (Phase 7A.opt F.4).

use std::sync::Arc;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

use sentinel_proto::members::v1 as proto_members;
use sentinel_proto::welcome::v1 as proto_welcome;

#[derive(Debug)]
#[allow(dead_code)]
pub struct WelcomeConfig {
    pub guild_id: String,
    pub welcome_enabled: bool,
    pub welcome_channel_id: Option<String>,
    pub welcome_message: String,
    pub welcome_embed_color: String,
    pub welcome_dm_enabled: bool,
    pub welcome_dm_message: String,
    pub leave_enabled: bool,
    pub leave_channel_id: Option<String>,
    pub leave_message: String,
    pub rules_enabled: bool,
    pub rules_channel_id: Option<String>,
    pub rules_message: String,
    pub rules_role_id: Option<String>,
    pub rules_button_label: String,
    pub counter_enabled: bool,
    pub counter_channel_id: Option<String>,
    pub counter_format: String,
    pub anniversary_enabled: bool,
    pub anniversary_channel_id: Option<String>,
    pub anniversary_message: String,
    pub rejoin_message: String,
}

impl From<proto_welcome::WelcomeConfig> for WelcomeConfig {
    fn from(p: proto_welcome::WelcomeConfig) -> Self {
        Self {
            guild_id: p.guild_id,
            welcome_enabled: p.welcome_enabled,
            welcome_channel_id: p.welcome_channel_id,
            welcome_message: p.welcome_message,
            welcome_embed_color: p.welcome_embed_color,
            welcome_dm_enabled: p.welcome_dm_enabled,
            welcome_dm_message: p.welcome_dm_message,
            leave_enabled: p.leave_enabled,
            leave_channel_id: p.leave_channel_id,
            leave_message: p.leave_message,
            rules_enabled: p.rules_enabled,
            rules_channel_id: p.rules_channel_id,
            rules_message: p.rules_message,
            rules_role_id: p.rules_role_id,
            rules_button_label: p.rules_button_label,
            counter_enabled: p.counter_enabled,
            counter_channel_id: p.counter_channel_id,
            counter_format: p.counter_format,
            anniversary_enabled: p.anniversary_enabled,
            anniversary_channel_id: p.anniversary_channel_id,
            anniversary_message: p.anniversary_message,
            rejoin_message: p.rejoin_message,
        }
    }
}

pub struct WelcomeApiClient {
    // Conserve pour compat TypeMap (heartbeat reste HTTP).
    #[allow(dead_code)]
    pub base: Arc<BaseApiClient>,
    grpc: Arc<SentinelGrpcClient>,
}

impl WelcomeApiClient {
    pub fn new(base: Arc<BaseApiClient>, grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { base, grpc }
    }

    /// gRPC `WelcomeService.GetConfig` (Phase 7A.opt F.4).
    pub async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfig, String> {
        let req = proto_welcome::GetConfigRequest {
            guild_id: guild_id.to_string(),
        };
        let mut client = self.grpc.welcome();
        let cfg = self
            .grpc
            .guarded(|| async move { client.get_config(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(|e| match e {
                GrpcCallError::Unavailable => "API indisponible (circuit breaker ouvert)".to_string(),
                GrpcCallError::Status(s) => format!("gRPC {:?}: {}", s.code(), s.message()),
                GrpcCallError::Transport(t) => format!("transport gRPC: {t}"),
            })?;
        Ok(cfg.into())
    }

    /// gRPC `MembersService.GetMember` (hot path).
    /// Renvoie `false` si le membre n'existe pas (parite avec l'ancien 404 HTTP).
    pub async fn is_known_member(&self, guild_id: &str, user_id: &str) -> bool {
        let req = proto_members::GetMemberRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.members();
        let result = self
            .grpc
            .guarded(|| async move { client.get_member(req).await.map(|r| r.into_inner()) })
            .await;
        match result {
            Ok(resp) => resp.member.is_some(),
            Err(_) => false,
        }
    }
}
