//! Client API du module security.
//!
//! Migration gRPC complete :
//! - Security events (report + list) -> `SecurityService`
//! - Members CRUD (sync, register, remove, update) -> `MembersService`

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::SentinelGrpcClient;

use sentinel_proto::members::v1 as proto_members;
use sentinel_proto::security::v1 as proto_security;

#[derive(Debug, Serialize)]
pub struct SecurityEvent {
    pub guild_id: String,
    pub event_type: String,
    pub severity: String,
    pub description: String,
    pub user_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberPayload {
    pub guild_id: String,
    pub user_id: UserId,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: serde_json::Value,
    pub joined_at: Option<DateTime<Utc>>,
    pub account_created: Option<DateTime<Utc>>,
    pub is_bot: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SyncMembersPayload {
    pub guild_id: String,
    pub members: Vec<MemberPayload>,
}

#[derive(Debug, Serialize)]
pub struct UpdateMemberPayload {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: Option<serde_json::Value>,
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

    // ── Security events (gRPC) ──

    pub async fn list_events(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let req = proto_security::ListEventsRequest {
            guild_id: Some(guild_id.to_string()),
        };
        let mut client = self.grpc.security();
        let list = self
            .grpc
            .guarded(|| async move { client.list_events(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        // Le proto ne porte pas de champ `limit` : on tronque cote client.
        // Les evenements sont supposes etre retournes les plus recents d'abord.
        Ok(list
            .events
            .into_iter()
            .take(limit as usize)
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "guild_id": e.guild_id,
                    "event_type": e.event_type,
                    "severity": e.severity,
                    "description": e.description,
                    "user_ids": e.user_ids,
                    "created_at": e.created_at,
                })
            })
            .collect())
    }

    pub async fn report_event(&self, event: &SecurityEvent) -> Result<(), String> {
        let req = proto_security::ReportEventRequest {
            guild_id: event.guild_id.clone(),
            event_type: event.event_type.clone(),
            severity: event.severity.clone(),
            description: event.description.clone(),
            user_ids: event.user_ids.clone(),
        };
        let mut client = self.grpc.security();
        self.grpc
            .guarded(|| async move { client.report_event(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    // ── Members CRUD (gRPC) ──

    pub async fn sync_members(&self, payload: &SyncMembersPayload) -> Result<(), String> {
        let req = proto_members::SyncMembersRequest {
            guild_id: payload.guild_id.clone(),
            members: payload
                .members
                .iter()
                .map(member_payload_to_proto)
                .collect::<Result<Vec<_>, String>>()?,
        };
        let mut client = self.grpc.members();
        self.grpc
            .guarded(|| async move { client.sync_members(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn register_member(&self, member: &MemberPayload) -> Result<(), String> {
        let req = proto_members::RegisterMemberRequest {
            member: Some(member_payload_to_proto(member)?),
        };
        let mut client = self.grpc.members();
        self.grpc
            .guarded(|| async move { client.register_member(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn remove_member(&self, guild_id: &str, user_id: &str) -> Result<(), String> {
        let req = proto_members::RemoveMemberRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.members();
        self.grpc
            .guarded(|| async move { client.remove_member(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn update_member(
        &self,
        guild_id: &str,
        user_id: &str,
        payload: &UpdateMemberPayload,
    ) -> Result<(), String> {
        let roles_json = match &payload.roles {
            Some(v) => Some(
                serde_json::to_string(v)
                    .map_err(|e| format!("serialisation roles: {e}"))?,
            ),
            None => None,
        };
        let req = proto_members::UpdateMemberRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: payload.username.clone(),
            display_name: payload.display_name.clone(),
            avatar: payload.avatar.clone(),
            roles_json,
        };
        let mut client = self.grpc.members();
        self.grpc
            .guarded(|| async move { client.update_member(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    // ── Analyse nouveau membre (gRPC) ──

    pub async fn analyze_new_member(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        has_avatar: bool,
        account_created_timestamp: i64,
        is_bot: bool,
        recent_joins: Vec<RecentJoinEntry>,
    ) -> Result<SecurityDecisionResponse, String> {
        let req = proto_security::AnalyzeNewMemberRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            has_avatar,
            account_created_timestamp,
            is_bot,
            recent_joins: recent_joins
                .into_iter()
                .map(|j| proto_security::RecentJoinEntry {
                    username: j.username,
                    has_avatar: j.has_avatar,
                    account_created_timestamp: j.account_created_timestamp,
                })
                .collect(),
        };
        let mut client = self.grpc.security();
        let resp = self
            .grpc
            .guarded(|| async move {
                client.analyze_new_member(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(SecurityDecisionResponse {
            is_raid: resp.is_raid,
            raid_score: resp.raid_score,
            is_suspicious_account: resp.is_suspicious_account,
            is_alt_account: resp.is_alt_account,
            alt_similar_to: resp.alt_similar_to,
            quarantine: resp.quarantine,
            send_captcha: resp.send_captcha,
            activate_lockdown: resp.activate_lockdown,
            slowmode_secs: resp.slowmode_secs,
            event_type: resp.event_type,
            event_description: resp.event_description,
        })
    }
}

// ── DTOs ──

#[derive(Debug, Clone)]
pub struct RecentJoinEntry {
    pub username: String,
    pub has_avatar: bool,
    pub account_created_timestamp: i64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SecurityDecisionResponse {
    pub is_raid: bool,
    pub raid_score: u32,
    pub is_suspicious_account: bool,
    pub is_alt_account: bool,
    pub alt_similar_to: String,
    pub quarantine: bool,
    pub send_captcha: bool,
    pub activate_lockdown: bool,
    pub slowmode_secs: u32,
    pub event_type: String,
    pub event_description: String,
}

fn member_payload_to_proto(p: &MemberPayload) -> Result<proto_members::GuildMember, String> {
    let roles_json = serde_json::to_string(&p.roles)
        .map_err(|e| format!("serialisation roles: {e}"))?;
    Ok(proto_members::GuildMember {
        guild_id: p.guild_id.clone(),
        user_id: p.user_id.clone(),
        username: p.username.clone(),
        display_name: p.display_name.clone(),
        avatar: p.avatar.clone(),
        roles_json,
        joined_at: p.joined_at.map(|d| d.to_rfc3339()),
        account_created: p.account_created.map(|d| d.to_rfc3339()),
        is_bot: p.is_bot,
        last_seen_at: p.last_seen_at.map(|d| d.to_rfc3339()),
    })
}

use sentinel_shared::grpc_client::grpc_err_to_string;
use sentinel_api::domain::entities::system::discord_ids::UserId;
