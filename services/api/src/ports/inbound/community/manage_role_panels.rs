use async_trait::async_trait;

use crate::domain::entities::community::role_panel::AutoRole;
use crate::domain::entities::community::role_panel::RolePanel;
use crate::domain::entities::community::role_panel::RolePanelDetail;
use crate::domain::errors::DomainError;

pub struct CreateRolePanelCommand {
    pub guild_id: String,
    pub channel_id: String,
    pub title: String,
    pub description: String,
    pub mode: String,
    pub max_roles: Option<i32>,
    pub entries: Vec<CreateRolePanelEntryCommand>,
}

pub struct CreateRolePanelEntryCommand {
    pub role_id: String,
    pub role_name: String,
    pub emoji: Option<String>,
    pub label: String,
    pub style: String,
    pub position: i32,
}

pub struct SetMessageIdCommand {
    pub panel_id: String,
    pub message_id: String,
}

pub struct CreateAutoRoleCommand {
    pub guild_id: String,
    pub role_id: String,
    pub role_name: String,
    pub delay_secs: i32,
}

#[async_trait]
pub trait ManageRolePanelsUseCase: Send + Sync {
    async fn create_panel(&self, cmd: CreateRolePanelCommand) -> Result<RolePanelDetail, DomainError>;
    async fn get_panel(&self, panel_id: &str) -> Result<RolePanelDetail, DomainError>;
    async fn get_panel_by_message(&self, message_id: &str) -> Result<Option<RolePanelDetail>, DomainError>;
    async fn list_panels(&self, guild_id: &str) -> Result<Vec<RolePanel>, DomainError>;
    async fn set_message_id(&self, cmd: SetMessageIdCommand) -> Result<(), DomainError>;
    async fn delete_panel(&self, panel_id: &str) -> Result<(), DomainError>;
    async fn list_auto_roles(&self, guild_id: &str) -> Result<Vec<AutoRole>, DomainError>;
    async fn add_auto_role(&self, cmd: CreateAutoRoleCommand) -> Result<AutoRole, DomainError>;
    async fn delete_auto_role(&self, guild_id: &str, role_id: &str) -> Result<(), DomainError>;
}
