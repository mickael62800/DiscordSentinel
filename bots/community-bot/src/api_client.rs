//! Client API du community-bot.
//!
//! Phase 7A — Migration gRPC :
//! - Role panels (5 methodes : get_panel, get_panel_by_message, list_panels,
//!   set_message_id, get_auto_roles) -> `RolePanelsService` partage avec
//!   roles-bot.
//! - Sponsorships et temp-roles (write/list/delete) restent en HTTP : pas de
//!   use case unifie cote API, repos directs.
//!
//! Surface publique inchangee.

use std::sync::Arc;

use serde::Deserialize;
use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

use sentinel_proto::roles::v1 as proto;

// ── DTOs (surface inchangee) ──

#[derive(Debug, Deserialize)]
pub struct TempRoleApiEntry {
    pub guild_id: String,
    pub user_id: String,
    pub role_id: String,
    pub expires_at: String,
}

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

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AutoRole {
    pub role_id: String,
    pub role_name: String,
    pub delay_secs: i32,
    pub enabled: bool,
}

pub struct ApiClient {
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

    pub async fn get_auto_roles(&self, guild_id: &str) -> Result<Vec<AutoRole>, String> {
        let req = proto::ListAutoRolesRequest {
            guild_id: guild_id.to_string(),
        };
        let mut client = self.grpc.role_panels();
        let list = self
            .grpc
            .guarded(|| async move {
                client.list_auto_roles(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list.roles.into_iter().map(proto_auto_role_to_dto).collect())
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

    // ── Sponsorships (HTTP fire-and-forget — pas de RPC v1) ──

    pub async fn create_sponsorship(
        &self,
        guild_id: &str,
        sponsor_id: &str,
        sponsored_id: &str,
    ) {
        self.base
            .post_fire_and_forget(
                "/api/sponsorships",
                &serde_json::json!({
                    "guild_id": guild_id,
                    "sponsor_id": sponsor_id,
                    "sponsored_id": sponsored_id,
                }),
            )
            .await;
    }

    // ── Temp Roles (HTTP — repos direct cote API, pas de RPC v1) ──

    pub async fn create_temp_role(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
        expires_at: &str,
    ) {
        self.base
            .post_fire_and_forget(
                "/api/temp-roles",
                &serde_json::json!({
                    "guild_id": guild_id,
                    "user_id": user_id,
                    "role_id": role_id,
                    "expires_at": expires_at,
                }),
            )
            .await;
    }

    pub async fn list_temp_roles(
        &self,
        guild_id: &str,
    ) -> Result<Vec<TempRoleApiEntry>, String> {
        self.base
            .get_json(&format!("/api/temp-roles/{}", guild_id))
            .await
    }

    pub async fn delete_temp_role(&self, guild_id: &str, user_id: &str, role_id: &str) {
        let req = self.base.client().delete(format!(
            "{}/api/temp-roles/{}/{}/{}",
            self.base.base_url(),
            guild_id,
            user_id,
            role_id
        ));
        if let Err(e) = self.base.auth(req).send().await {
            tracing::warn!(error = %e, "Failed to delete temp role");
        }
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

fn proto_auto_role_to_dto(r: proto::AutoRole) -> AutoRole {
    AutoRole {
        role_id: r.role_id,
        role_name: r.role_name,
        delay_secs: r.delay_secs,
        enabled: r.enabled,
    }
}

fn grpc_err_to_string(e: GrpcCallError) -> String {
    match e {
        GrpcCallError::Unavailable => "API indisponible (circuit breaker ouvert)".to_string(),
        GrpcCallError::Status(s) => format!("gRPC {:?}: {}", s.code(), s.message()),
        GrpcCallError::Transport(t) => format!("transport gRPC: {t}"),
    }
}
