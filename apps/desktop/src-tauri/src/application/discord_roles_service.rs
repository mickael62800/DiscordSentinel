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
}
