//! Client API du voice-bot.
//!
//! Phase 7A — Migration gRPC :
//! - VoiceChannels CRUD (list, create, delete, update, get, transfer,
//!   add_co_admin, add_to_whitelist, ban_user) -> `VoiceChannelsService`
//! - `log_moderation_action` -> reuse `ModerationService.LogAction`
//!
//! ## Note d'implementation
//!
//! Le voice-bot construit un nouvel `ApiClient` a chaque interaction (27
//! call sites repartis dans 10+ fichiers). Pour eviter de toucher chaque
//! site, le `SentinelGrpcClient` est stocke dans un `OnceLock` global
//! initialise depuis `main.rs` via [`init_grpc`]. `ApiClient::new(base)`
//! garde sa signature originale et lit le client depuis le static.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

use sentinel_proto::moderation::v1 as proto_mod;
use sentinel_proto::voice::v1 as proto;

// Phase 7A opt D.2 : passage du `OnceLock<Arc<SentinelGrpcClient>>` global
// au pattern classique (grpc field dans le struct, fourni par le TypeMap
// Serenity via `from_data`). Supprime le state global, rend l'ApiClient
// testable, et aligne voice-bot sur les 10 autres bots migres.

// ── Request DTOs (surface inchangee) ──

#[derive(Debug, Serialize)]
pub struct CreateVoiceChannelRequest {
    pub guild_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub channel_id: String,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    pub kind: String,
    pub visibility: String,
    pub queue_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct UpdateVoiceChannelRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_limit: Option<Option<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_channel_id: Option<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct TransferOwnershipRequest {
    pub new_owner_id: String,
    pub new_owner_name: String,
}

#[derive(Debug, Serialize)]
pub struct AddCoAdminRequest {
    pub user_id: String,
    pub user_name: String,
}

#[derive(Debug, Serialize)]
pub struct AddWhitelistRequest {
    pub guild_id: String,
    pub owner_id: String,
    pub target_id: String,
    pub target_name: String,
}

#[derive(Debug, Serialize)]
pub struct BanFromChannelRequest {
    pub user_id: String,
    pub user_name: String,
    pub banned_by: String,
    pub reason: Option<String>,
    pub duration_secs: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct LogModerationActionRequest {
    pub guild_id: String,
    pub channel_id: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub duration: Option<i64>,
}

// ── Response DTOs (surface inchangee) ──

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct VoiceChannelResponse {
    pub id: String,
    pub guild_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub channel_id: String,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    pub kind: String,
    pub visibility: String,
    pub queue_enabled: bool,
    pub locked: bool,
    pub member_limit: Option<i32>,
    pub status: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct VoiceChannelDetailResponse {
    pub channel: VoiceChannelResponse,
    pub co_admins: Vec<serde_json::Value>,
    pub bans: Vec<serde_json::Value>,
}

// ── Client ──

pub struct ApiClient {
    #[allow(dead_code)]
    pub base: Arc<BaseApiClient>,
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    /// Construction classique : prend le `BaseApiClient` HTTP (legacy, garde
    /// pour compat/heartbeat) et le `SentinelGrpcClient` (Phase 7A).
    pub fn new(base: Arc<BaseApiClient>, grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { base, grpc }
    }

    /// Helper : construit un `ApiClient` depuis le TypeMap Serenity. Renvoie
    /// `None` si l'un des deux clients n'a pas ete insere dans `main.rs`.
    /// Actuellement non utilise (les call sites fetchent base + grpc en ligne)
    /// mais garde pour de futurs refactors vers un pattern plus concis.
    #[allow(dead_code)]
    pub fn from_data(data: &serenity::prelude::TypeMap) -> Option<Self> {
        let base = data
            .get::<sentinel_shared::heartbeat::ApiClientKey>()?
            .clone();
        let grpc = data
            .get::<sentinel_shared::grpc_client::GrpcClientKey>()?
            .clone();
        Some(Self::new(base, grpc))
    }

    // ── Channels (gRPC) ──

    pub async fn list_channels(
        &self,
        guild_id: &str,
    ) -> Result<Vec<VoiceChannelResponse>, String> {
        let req = proto::ListChannelsRequest {
            guild_id: guild_id.to_string(),
        };
        let g = &self.grpc;
        let mut client = g.voice_channels();
        let list = g
            .guarded(|| async move {
                client.list_channels(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list.channels.into_iter().map(proto_to_response).collect())
    }

    pub async fn create_channel(
        &self,
        request: &CreateVoiceChannelRequest,
    ) -> Result<VoiceChannelResponse, String> {
        let req = proto::CreateChannelRequest {
            guild_id: request.guild_id.clone(),
            owner_id: request.owner_id.clone(),
            owner_name: request.owner_name.clone(),
            channel_id: request.channel_id.clone(),
            text_channel_id: request.text_channel_id.clone(),
            members_channel_id: request.members_channel_id.clone(),
            queue_channel_id: request.queue_channel_id.clone(),
            category_id: request.category_id.clone(),
            channel_name: request.channel_name.clone(),
            kind: request.kind.clone(),
            visibility: request.visibility.clone(),
            queue_enabled: request.queue_enabled,
        };
        let g = &self.grpc;
        let mut client = g.voice_channels();
        let c = g
            .guarded(|| async move {
                client.create_channel(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(proto_to_response(c))
    }

    pub async fn delete_channel(&self, channel_id: &str) -> Result<(), String> {
        let req = proto::DeleteChannelRequest {
            channel_id: channel_id.to_string(),
        };
        let g = &self.grpc;
        let mut client = g.voice_channels();
        g.guarded(|| async move { client.delete_channel(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn update_channel(
        &self,
        channel_id: &str,
        request: &UpdateVoiceChannelRequest,
    ) -> Result<(), String> {
        let req = proto::UpdateChannelRequest {
            channel_id: channel_id.to_string(),
            visibility: request.visibility.clone(),
            locked: request.locked,
            queue_enabled: request.queue_enabled,
            name: request.name.clone(),
            status: request.status.clone(),
            member_limit: request.member_limit.map(|opt| proto::MemberLimitUpdate { value: opt }),
            queue_channel_id: request
                .queue_channel_id
                .clone()
                .map(|opt| proto::QueueChannelUpdate { value: opt }),
        };
        let g = &self.grpc;
        let mut client = g.voice_channels();
        g.guarded(|| async move { client.update_channel(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn get_channel(
        &self,
        channel_id: &str,
    ) -> Result<Option<VoiceChannelResponse>, String> {
        let req = proto::GetChannelRequest {
            channel_id: channel_id.to_string(),
        };
        let g = &self.grpc;
        let mut client = g.voice_channels();
        let resp = g
            .guarded(|| async move { client.get_channel(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(resp.channel.map(proto_to_response))
    }

    // ── Transfer ──

    pub async fn transfer_ownership(
        &self,
        channel_id: &str,
        request: &TransferOwnershipRequest,
    ) -> Result<(), String> {
        let req = proto::TransferOwnershipRequest {
            channel_id: channel_id.to_string(),
            new_owner_id: request.new_owner_id.clone(),
            new_owner_name: request.new_owner_name.clone(),
        };
        let g = &self.grpc;
        let mut client = g.voice_channels();
        g.guarded(|| async move { client.transfer_ownership(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    // ── Co-admins ──

    pub async fn add_co_admin(
        &self,
        channel_id: &str,
        request: &AddCoAdminRequest,
    ) -> Result<(), String> {
        let req = proto::AddCoAdminRequest {
            channel_id: channel_id.to_string(),
            user_id: request.user_id.clone(),
            user_name: request.user_name.clone(),
        };
        let g = &self.grpc;
        let mut client = g.voice_channels();
        g.guarded(|| async move { client.add_co_admin(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    // ── Whitelist ──

    pub async fn add_to_whitelist(&self, request: &AddWhitelistRequest) -> Result<(), String> {
        let req = proto::AddToWhitelistRequest {
            guild_id: request.guild_id.clone(),
            owner_id: request.owner_id.clone(),
            target_id: request.target_id.clone(),
            target_name: request.target_name.clone(),
        };
        let g = &self.grpc;
        let mut client = g.voice_channels();
        g.guarded(|| async move { client.add_to_whitelist(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    // ── Bans ──

    pub async fn ban_user(
        &self,
        channel_id: &str,
        request: &BanFromChannelRequest,
    ) -> Result<(), String> {
        let req = proto::BanFromChannelRequest {
            channel_id: channel_id.to_string(),
            user_id: request.user_id.clone(),
            user_name: request.user_name.clone(),
            banned_by: request.banned_by.clone(),
            reason: request.reason.clone(),
            duration_secs: request.duration_secs,
        };
        let g = &self.grpc;
        let mut client = g.voice_channels();
        g.guarded(|| async move { client.ban_from_channel(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    // ── Moderation log (reuse ModerationService.LogAction) ──

    pub async fn log_moderation_action(
        &self,
        request: &LogModerationActionRequest,
    ) -> Result<(), String> {
        let req = proto_mod::LogActionRequest {
            guild_id: request.guild_id.clone(),
            channel_id: request.channel_id.clone(),
            moderator_id: request.moderator_id.clone(),
            moderator_name: request.moderator_name.clone(),
            target_id: request.target_id.clone(),
            target_name: request.target_name.clone(),
            action_type: request.action_type.clone(),
            reason: request.reason.clone(),
            gravity: None,
            duration: request.duration.map(|d| d as u64),
        };
        let g = &self.grpc;
        let mut client = g.moderation();
        g.guarded(|| async move { client.log_action(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }
}

fn proto_to_response(c: proto::VoiceChannel) -> VoiceChannelResponse {
    VoiceChannelResponse {
        id: c.id,
        guild_id: c.guild_id,
        owner_id: c.owner_id,
        owner_name: c.owner_name,
        channel_id: c.channel_id,
        text_channel_id: c.text_channel_id,
        members_channel_id: c.members_channel_id,
        queue_channel_id: c.queue_channel_id,
        category_id: c.category_id,
        channel_name: c.channel_name,
        kind: c.kind,
        visibility: c.visibility,
        queue_enabled: c.queue_enabled,
        locked: c.locked,
        member_limit: c.member_limit,
        status: c.status,
        created_at: c.created_at,
    }
}

fn grpc_err_to_string(e: GrpcCallError) -> String {
    match e {
        GrpcCallError::Unavailable => "API indisponible (circuit breaker ouvert)".to_string(),
        GrpcCallError::Status(s) => format!("gRPC {:?}: {}", s.code(), s.message()),
        GrpcCallError::Transport(t) => format!("transport gRPC: {t}"),
    }
}
