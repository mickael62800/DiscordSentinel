use std::sync::Arc;

use crate::domain::entities::{BotDefinition, BotGuildConfig};
use crate::domain::ports::BotConfigRepository;

pub struct BotConfigService {
    repo: Arc<dyn BotConfigRepository>,
}

impl BotConfigService {
    pub fn new(repo: Arc<dyn BotConfigRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_definitions(&self) -> Result<Vec<BotDefinition>, String> {
        self.repo.get_definitions().await
    }

    pub async fn get_guild_config(&self, guild_id: String) -> Result<Vec<BotGuildConfig>, String> {
        self.repo.get_guild_config(guild_id).await
    }

    pub async fn set_config(&self, guild_id: String, bot_name: String, key: String, value: String) -> Result<(), String> {
        self.repo.set_config(guild_id, bot_name, key, value).await
    }

    pub async fn delete_config(&self, guild_id: String, bot_name: String, key: String) -> Result<(), String> {
        self.repo.delete_config(guild_id, bot_name, key).await
    }
}
