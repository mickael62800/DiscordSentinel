//! Implementation gRPC du `RolePanelsService` (Phase 7A).
//! Wrappe `ManageRolePanelsUseCase`. Partage par community-bot et roles-bot.

use std::sync::Arc;

use tonic::Request;
use tonic::Response;
use tonic::Status;
use sentinel_proto::roles::v1 as proto;
use sentinel_proto::roles::v1::role_panels_service_server::RolePanelsService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::community::role_panel::AutoRole;
use crate::domain::entities::system::discord_role::DiscordRole;
use crate::domain::entities::community::role_panel::RolePanel;
use crate::domain::entities::community::role_panel::RolePanelDetail;
use crate::domain::entities::community::role_panel::RolePanelEntry;
use crate::ports::inbound::community::manage_role_panels::SetMessageIdCommand;
use crate::ports::inbound::community::manage_role_panels::ManageRolePanelsUseCase;
use crate::ports::outbound::community::discord_role_repository::DiscordRoleRepository;

pub struct RolePanelsGrpc {
    pub uc: Arc<dyn ManageRolePanelsUseCase>,
    /// Phase 7A.opt F.5 — pour SyncDiscordRoles (pas de use case unifie).
    pub discord_role_repo: Arc<dyn DiscordRoleRepository>,
}

#[tonic::async_trait]
impl RolePanelsService for RolePanelsGrpc {
    async fn get_panel(
        &self,
        request: Request<proto::GetPanelRequest>,
    ) -> Result<Response<proto::GetPanelResponse>, Status> {
        let req = request.into_inner();
        // L'API HTTP retournait 404 -> Option::None ; on garde le meme contrat
        // ici en convertissant NotFound -> panel: None.
        let result = self.uc.get_panel(&req.panel_id).await;
        match result {
            Ok(d) => Ok(Response::new(proto::GetPanelResponse {
                panel: Some(role_panel_detail_to_proto(d)),
            })),
            Err(crate::domain::errors::DomainError::NotFound(_)) => {
                Ok(Response::new(proto::GetPanelResponse { panel: None }))
            }
            Err(e) => Err(domain_to_status(e)),
        }
    }

    async fn get_panel_by_message(
        &self,
        request: Request<proto::GetPanelByMessageRequest>,
    ) -> Result<Response<proto::GetPanelResponse>, Status> {
        let req = request.into_inner();
        let detail = self
            .uc
            .get_panel_by_message(&req.message_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::GetPanelResponse {
            panel: detail.map(role_panel_detail_to_proto),
        }))
    }

    async fn list_panels(
        &self,
        request: Request<proto::ListPanelsRequest>,
    ) -> Result<Response<proto::RolePanelList>, Status> {
        let req = request.into_inner();
        let panels = self
            .uc
            .list_panels(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::RolePanelList {
            panels: panels.into_iter().map(role_panel_to_proto).collect(),
        }))
    }

    async fn set_message_id(
        &self,
        request: Request<proto::SetMessageIdRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .set_message_id(SetMessageIdCommand {
                panel_id: req.panel_id,
                message_id: req.message_id.into(),
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn list_auto_roles(
        &self,
        request: Request<proto::ListAutoRolesRequest>,
    ) -> Result<Response<proto::AutoRoleList>, Status> {
        let req = request.into_inner();
        let roles = self
            .uc
            .list_auto_roles(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::AutoRoleList {
            roles: roles.into_iter().map(auto_role_to_proto).collect(),
        }))
    }

    // Phase 7A.opt F.5 — sync batch des roles Discord vers l'API.
    async fn sync_discord_roles(
        &self,
        request: Request<proto::SyncDiscordRolesRequest>,
    ) -> Result<Response<proto::SyncDiscordRolesResponse>, Status> {
        let req = request.into_inner();
        let count = req.roles.len() as u64;
        let roles: Vec<DiscordRole> = req
            .roles
            .into_iter()
            .map(|r| DiscordRole {
                id: r.id,
                guild_id: req.guild_id.clone(),
                name: r.name,
                color: r.color,
                position: r.position,
                // Parse String -> i64 (bitfield Discord, fallback 0).
                permissions: r.permissions.parse::<i64>().unwrap_or(0),
                mentionable: r.mentionable,
                managed: r.managed,
                icon: r.icon,
                member_count: r.member_count,
                synced_at: chrono::Utc::now(),
            })
            .collect();
        self.discord_role_repo
            .sync_roles(&req.guild_id, roles)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SyncDiscordRolesResponse { synced: count }))
    }
}

fn role_panel_to_proto(p: RolePanel) -> proto::RolePanel {
    proto::RolePanel {
        id: p.id.to_string(),
        guild_id: p.guild_id,
        channel_id: p.channel_id.into(),
        message_id: p.message_id,
        title: p.title,
        description: p.description,
        mode: p.mode,
        max_roles: p.max_roles,
        enabled: p.enabled,
    }
}

fn role_panel_entry_to_proto(e: RolePanelEntry) -> proto::RolePanelEntry {
    proto::RolePanelEntry {
        id: e.id.to_string(),
        role_id: e.role_id.into(),
        role_name: e.role_name,
        emoji: e.emoji,
        label: e.label,
        style: e.style,
        position: e.position,
    }
}

fn role_panel_detail_to_proto(d: RolePanelDetail) -> proto::RolePanelDetail {
    proto::RolePanelDetail {
        panel: Some(role_panel_to_proto(d.panel)),
        entries: d.entries.into_iter().map(role_panel_entry_to_proto).collect(),
    }
}

fn auto_role_to_proto(r: AutoRole) -> proto::AutoRole {
    proto::AutoRole {
        role_id: r.role_id.into(),
        role_name: r.role_name,
        delay_secs: r.delay_secs,
        enabled: r.enabled,
    }
}


#[cfg(test)]
#[path = "tests/roles.rs"]
mod tests;
