use std::sync::Arc;
use crate::domain::entities::{AutoRoleConfig, RolePanel, RolePanelDetail};
use crate::domain::ports::RolePanelsRepository;

pub struct RolePanelsService {
    repo: Arc<dyn RolePanelsRepository>,
}

impl RolePanelsService {
    pub fn new(repo: Arc<dyn RolePanelsRepository>) -> Self { Self { repo } }

    pub async fn get_panels(&self, guild_id: String) -> Result<Vec<RolePanel>, String> {
        self.repo.get_panels(guild_id).await
    }
    pub async fn get_panel(&self, panel_id: String) -> Result<RolePanelDetail, String> {
        self.repo.get_panel(panel_id).await
    }
    pub async fn get_auto_roles(&self, guild_id: String) -> Result<Vec<AutoRoleConfig>, String> {
        self.repo.get_auto_roles(guild_id).await
    }
}
