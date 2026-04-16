//! Client API du roles-bot.
//!
//! Phase 7A — Migration gRPC : memes RPCs que community-bot via
//! `RolePanelsService`. `sync_discord_roles` reste HTTP (pas de use case
//! unifie cote API).

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

use sentinel_proto::roles::v1 as proto;

#[derive(Debug, Deserialize)]
pub struct RolePanelDetail {
    pub panel: RolePanel,
    pub entries: Vec<RolePanelEntry>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RolePanel {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: Option<String>,
    pub title: String,
    pub description: String,
    pub mode: String,
    pub max_roles: Option<i32>,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RolePanelEntry {
    pub id: String,
    pub role_id: String,
    pub role_name: String,
    pub emoji: Option<String>,
    pub label: String,
    pub style: String,
    pub position: i32,
}

#[derive(Debug, Serialize)]
pub struct SyncRole {
    pub id: String,
    pub name: String,
    pub color: i32,
    pub position: i32,
    pub permissions: String,
    pub mentionable: bool,
    pub managed: bool,
    pub icon: Option<String>,
    pub member_count: i32,
}

pub struct ApiClient {
    // Phase 7A.opt F.5 : `base` n'est plus utilise — sync_discord_roles est
    // migre en gRPC. Conserve pour le heartbeat via TypeMap.
    #[allow(dead_code)]
    pub base: Arc<BaseApiClient>,
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>, grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { base, grpc }
    }

    // ── Role panels (gRPC) ──

    #[allow(dead_code)]
    pub async fn get_panel_by_message(
        &self,
        message_id: &str,
    ) -> Result<Option<RolePanelDetail>, String> {
        let req = proto::GetPanelByMessageRequest {
            message_id: message_id.to_string(),
        };
        let mut client = self.grpc.role_panels();
        let resp = self
            .grpc
            .guarded(|| async move {
                client.get_panel_by_message(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(resp.panel.map(proto_detail_to_dto))
    }

    pub async fn set_message_id(
        &self,
        panel_id: &str,
        message_id: &str,
    ) -> Result<(), String> {
        let req = proto::SetMessageIdRequest {
            panel_id: panel_id.to_string(),
            message_id: message_id.to_string(),
        };
        let mut client = self.grpc.role_panels();
        self.grpc
            .guarded(|| async move { client.set_message_id(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn list_panels(&self, guild_id: &str) -> Result<Vec<RolePanel>, String> {
        let req = proto::ListPanelsRequest {
            guild_id: guild_id.to_string(),
        };
        let mut client = self.grpc.role_panels();
        let list = self
            .grpc
            .guarded(|| async move {
                client.list_panels(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list.panels.into_iter().map(proto_panel_to_dto).collect())
    }

    pub async fn get_panel(&self, panel_id: &str) -> Result<Option<RolePanelDetail>, String> {
        let req = proto::GetPanelRequest {
            panel_id: panel_id.to_string(),
        };
        let mut client = self.grpc.role_panels();
        let resp = self
            .grpc
            .guarded(|| async move { client.get_panel(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(resp.panel.map(proto_detail_to_dto))
    }

    // ── Phase 7A.opt F.5 — Discord roles sync en gRPC ──

    /// gRPC `RolePanelsService.SyncDiscordRoles` (Phase 7A.opt F.5).
    pub async fn sync_discord_roles(
        &self,
        guild_id: &str,
        roles: Vec<SyncRole>,
    ) -> Result<(), String> {
        let req = proto::SyncDiscordRolesRequest {
            guild_id: guild_id.to_string(),
            roles: roles
                .into_iter()
                .map(|r| proto::SyncDiscordRole {
                    id: r.id,
                    name: r.name,
                    color: r.color,
                    position: r.position,
                    permissions: r.permissions,
                    mentionable: r.mentionable,
                    managed: r.managed,
                    icon: r.icon,
                    member_count: r.member_count,
                })
                .collect(),
        };
        let mut client = self.grpc.role_panels();
        self.grpc
            .guarded(|| async move { client.sync_discord_roles(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }
}

// ── Helpers proto -> DTO ──

fn proto_panel_to_dto(p: proto::RolePanel) -> RolePanel {
    RolePanel {
        id: p.id,
        guild_id: p.guild_id,
        channel_id: p.channel_id,
        message_id: p.message_id,
        title: p.title,
        description: p.description,
        mode: p.mode,
        max_roles: p.max_roles,
        enabled: p.enabled,
    }
}

fn proto_entry_to_dto(e: proto::RolePanelEntry) -> RolePanelEntry {
    RolePanelEntry {
        id: e.id,
        role_id: e.role_id,
        role_name: e.role_name,
        emoji: e.emoji,
        label: e.label,
        style: e.style,
        position: e.position,
    }
}

fn proto_detail_to_dto(d: proto::RolePanelDetail) -> RolePanelDetail {
    RolePanelDetail {
        panel: d.panel.map(proto_panel_to_dto).unwrap_or(RolePanel {
            id: String::new(),
            guild_id: String::new(),
            channel_id: String::new(),
            message_id: None,
            title: String::new(),
            description: String::new(),
            mode: String::new(),
            max_roles: None,
            enabled: false,
        }),
        entries: d.entries.into_iter().map(proto_entry_to_dto).collect(),
    }
}

fn grpc_err_to_string(e: GrpcCallError) -> String {
    match e {
        GrpcCallError::Unavailable => "API indisponible (circuit breaker ouvert)".to_string(),
        GrpcCallError::Status(s) => format!("gRPC {:?}: {}", s.code(), s.message()),
        GrpcCallError::Transport(t) => format!("transport gRPC: {t}"),
    }
}
