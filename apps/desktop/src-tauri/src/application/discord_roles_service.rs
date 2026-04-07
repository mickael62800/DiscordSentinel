use std::sync::Arc;
use crate::domain::entities::DiscordRole;
use crate::domain::ports::DiscordRolesRepository;

pub struct DiscordRolesService {
    repo: Arc<dyn DiscordRolesRepository>,
}

impl DiscordRolesService {
    pub fn new(repo: Arc<dyn DiscordRolesRepository>) -> Self { Self { repo } }

    pub async fn get_discord_roles(&self, guild_id: String) -> Result<Vec<DiscordRole>, String> {
        self.repo.get_discord_roles(guild_id).await
    }

    pub async fn create_role(&self, guild_id: String, name: String, color: u32, permissions: Option<String>) -> Result<serde_json::Value, String> {
        self.repo.create_discord_role(guild_id, name, color, permissions).await
    }

    pub async fn edit_role(&self, guild_id: String, role_id: String, name: Option<String>, color: Option<u32>, permissions: Option<String>, mentionable: Option<bool>) -> Result<serde_json::Value, String> {
        self.repo.edit_discord_role(guild_id, role_id, name, color, permissions, mentionable).await
    }

    pub async fn delete_role(&self, guild_id: String, role_id: String) -> Result<(), String> {
        self.repo.delete_discord_role(guild_id, role_id).await
    }
}
