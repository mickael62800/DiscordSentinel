use std::sync::Arc;

use crate::domain::entities::{GuildUserRole, MyRole};
use crate::domain::ports::RbacRepository;

pub struct RbacService {
    repo: Arc<dyn RbacRepository>,
}

impl RbacService {
    pub fn new(repo: Arc<dyn RbacRepository>) -> Self {
        Self { repo }
    }

    pub async fn list_guild_users(&self, guild_id: String) -> Result<Vec<GuildUserRole>, String> {
        self.repo.list_guild_users(guild_id).await
    }

    pub async fn get_my_role(&self, guild_id: String) -> Result<MyRole, String> {
        self.repo.get_my_role(guild_id).await
    }

    pub async fn grant_role(
        &self,
        guild_id: String,
        user_id: String,
        role: String,
        display_name: Option<String>,
    ) -> Result<(), String> {
        self.repo.grant_role(guild_id, user_id, role, display_name).await
    }

    pub async fn update_role(
        &self,
        guild_id: String,
        user_id: String,
        role: String,
    ) -> Result<(), String> {
        self.repo.update_role(guild_id, user_id, role).await
    }

    pub async fn revoke_role(&self, guild_id: String, user_id: String) -> Result<(), String> {
        self.repo.revoke_role(guild_id, user_id).await
    }
}
