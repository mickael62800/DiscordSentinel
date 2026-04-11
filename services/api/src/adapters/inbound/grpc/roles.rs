//! Implementation gRPC du `RolePanelsService` (Phase 7A).
//! Wrappe `ManageRolePanelsUseCase`. Partage par community-bot et roles-bot.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use sentinel_proto::roles::v1 as proto;
use sentinel_proto::roles::v1::role_panels_service_server::RolePanelsService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::{AutoRole, DiscordRole, RolePanel, RolePanelDetail, RolePanelEntry};
use crate::ports::inbound::manage_role_panels::SetMessageIdCommand;
use crate::ports::inbound::ManageRolePanelsUseCase;
use crate::ports::outbound::DiscordRoleRepository;

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
                message_id: req.message_id,
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
        channel_id: p.channel_id,
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
        role_id: e.role_id,
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
        role_id: r.role_id,
        role_name: r.role_name,
        delay_secs: r.delay_secs,
        enabled: r.enabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    fn sample_panel() -> RolePanel {
        RolePanel {
            id: Uuid::nil(),
            guild_id: "g".into(),
            channel_id: "c".into(),
            message_id: Some("m".into()),
            title: "Roles".into(),
            description: "Choisis".into(),
            mode: "buttons".into(),
            max_roles: Some(3),
            enabled: true,
            created_at: ts(),
            updated_at: ts(),
        }
    }

    #[test]
    fn role_panel_to_proto_full_mapping() {
        let p = role_panel_to_proto(sample_panel());
        assert_eq!(p.id, Uuid::nil().to_string());
        assert_eq!(p.guild_id, "g");
        assert_eq!(p.channel_id, "c");
        assert_eq!(p.message_id.as_deref(), Some("m"));
        assert_eq!(p.title, "Roles");
        assert_eq!(p.mode, "buttons");
        assert_eq!(p.max_roles, Some(3));
        assert!(p.enabled);
    }

    #[test]
    fn role_panel_to_proto_optional_fields_none() {
        let mut panel = sample_panel();
        panel.message_id = None;
        panel.max_roles = None;
        panel.enabled = false;
        let p = role_panel_to_proto(panel);
        assert!(p.message_id.is_none());
        assert!(p.max_roles.is_none());
        assert!(!p.enabled);
    }

    #[test]
    fn role_panel_entry_to_proto_full_mapping() {
        let e = RolePanelEntry {
            id: Uuid::nil(),
            panel_id: Uuid::nil(),
            role_id: "r1".into(),
            role_name: "Gamer".into(),
            emoji: Some("🎮".into()),
            label: "Joueur".into(),
            style: "primary".into(),
            position: 2,
        };
        let p = role_panel_entry_to_proto(e);
        assert_eq!(p.role_id, "r1");
        assert_eq!(p.label, "Joueur");
        assert_eq!(p.style, "primary");
        assert_eq!(p.position, 2);
        assert_eq!(p.emoji.as_deref(), Some("🎮"));
    }

    #[test]
    fn role_panel_detail_to_proto_includes_entries() {
        let detail = RolePanelDetail {
            panel: sample_panel(),
            entries: vec![
                RolePanelEntry {
                    id: Uuid::nil(), panel_id: Uuid::nil(),
                    role_id: "a".into(), role_name: "A".into(),
                    emoji: None, label: "A".into(), style: "primary".into(), position: 0,
                },
                RolePanelEntry {
                    id: Uuid::nil(), panel_id: Uuid::nil(),
                    role_id: "b".into(), role_name: "B".into(),
                    emoji: None, label: "B".into(), style: "primary".into(), position: 1,
                },
            ],
        };
        let p = role_panel_detail_to_proto(detail);
        assert!(p.panel.is_some());
        assert_eq!(p.entries.len(), 2);
        assert_eq!(p.entries[0].role_id, "a");
        assert_eq!(p.entries[1].position, 1);
    }

    #[test]
    fn auto_role_to_proto_full_mapping() {
        let r = AutoRole {
            id: Uuid::nil(),
            guild_id: "g".into(),
            role_id: "r".into(),
            role_name: "Member".into(),
            delay_secs: 60,
            enabled: true,
        };
        let p = auto_role_to_proto(r);
        assert_eq!(p.role_id, "r");
        assert_eq!(p.role_name, "Member");
        assert_eq!(p.delay_secs, 60);
        assert!(p.enabled);
    }
}
