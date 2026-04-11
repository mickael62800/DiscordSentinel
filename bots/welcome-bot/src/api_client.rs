//! Client API du welcome-bot.
//!
//! Phase 7A — Migration gRPC :
//! - `is_known_member` -> `MembersService.GetMember` (hot path : a chaque
//!   nouveau membre rejoignant un serveur).
//! - `get_config` reste HTTP : `WelcomeConfig` est un blob de config sans
//!   use case unifie cote API (lecture rare, pas critique).

use std::sync::Arc;

use serde::Deserialize;
use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::SentinelGrpcClient;

use sentinel_proto::members::v1 as proto_members;

#[derive(Debug, Deserialize)]
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

pub struct WelcomeApiClient {
    pub base: Arc<BaseApiClient>,
    grpc: Arc<SentinelGrpcClient>,
}

impl WelcomeApiClient {
    pub fn new(base: Arc<BaseApiClient>, grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { base, grpc }
    }

    /// HTTP : pas de RPC v1 pour la welcome config (gros blob, lecture rare).
    pub async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfig, String> {
        self.base.get_json(&format!("/api/welcome/{guild_id}")).await
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
