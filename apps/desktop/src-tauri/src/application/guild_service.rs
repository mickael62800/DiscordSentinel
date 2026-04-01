use std::sync::Arc;

use crate::domain::entities::{Guild, GuildMember};
use crate::domain::ports::GuildRepository;

pub struct GuildService {
    repo: Arc<dyn GuildRepository>,
}

impl GuildService {
    pub fn new(repo: Arc<dyn GuildRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_guilds(&self) -> Result<Vec<Guild>, String> {
        self.repo.get_guilds().await
    }

    pub async fn get_guild_members(&self, guild_id: String) -> Result<Vec<GuildMember>, String> {
        self.repo.get_guild_members(guild_id).await
    }
}
