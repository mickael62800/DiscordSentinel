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

use sentinel_proto::community::v1 as proto_community;
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
    // Phase 7A.opt F.3 : `base` n'est plus utilise — tous les appels metier
    // du community-bot sont en gRPC (role panels + sponsorships + temp roles).
    // Conserve pour le heartbeat via TypeMap.
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

    // ── Phase 7A.opt F.3 — Sponsorships + Temp Roles en gRPC ──

    /// gRPC `CommunityService.CreateSponsorship`.
    pub async fn create_sponsorship(
        &self,
        guild_id: &str,
        sponsor_id: &str,
        sponsored_id: &str,
    ) {
        let req = proto_community::CreateSponsorshipRequest {
            guild_id: guild_id.to_string(),
            sponsor_id: sponsor_id.to_string(),
            sponsored_id: sponsored_id.to_string(),
        };
        let mut client = self.grpc.community();
        let _ = self
            .grpc
            .guarded(|| async move { client.create_sponsorship(req).await.map(|_| ()) })
            .await;
        // fire-and-forget : on ignore l'erreur (historique HTTP).
    }

    /// gRPC `CommunityService.CreateTempRole`.
    pub async fn create_temp_role(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
        expires_at: &str,
    ) {
        let req = proto_community::CreateTempRoleRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            role_id: role_id.to_string(),
            expires_at: expires_at.to_string(),
        };
        let mut client = self.grpc.community();
        let _ = self
            .grpc
            .guarded(|| async move { client.create_temp_role(req).await.map(|_| ()) })
            .await;
    }

    /// gRPC `CommunityService.ListTempRoles`.
    pub async fn list_temp_roles(
        &self,
        guild_id: &str,
    ) -> Result<Vec<TempRoleApiEntry>, String> {
        let req = proto_community::ListTempRolesRequest {
            guild_id: guild_id.to_string(),
        };
        let mut client = self.grpc.community();
        let list = self
            .grpc
            .guarded(|| async move { client.list_temp_roles(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list
            .roles
            .into_iter()
            .map(|r| TempRoleApiEntry {
                guild_id: r.guild_id,
                user_id: r.user_id,
                role_id: r.role_id,
                expires_at: r.expires_at,
            })
            .collect())
    }

    /// gRPC `CommunityService.DeleteTempRole`.
    pub async fn delete_temp_role(&self, guild_id: &str, user_id: &str, role_id: &str) {
        let req = proto_community::DeleteTempRoleRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            role_id: role_id.to_string(),
        };
        let mut client = self.grpc.community();
        if let Err(e) = self
            .grpc
            .guarded(|| async move { client.delete_temp_role(req).await.map(|_| ()) })
            .await
        {
            tracing::warn!(error = ?e, "Failed to delete temp role");
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
