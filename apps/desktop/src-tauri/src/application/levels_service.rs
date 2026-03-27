use std::sync::Arc;

use crate::domain::entities::{LevelConfig, LevelReward, UserLevel};
use crate::domain::ports::LevelRepository;

pub struct LevelsService {
    repo: Arc<dyn LevelRepository>,
}

impl LevelsService {
    pub fn new(repo: Arc<dyn LevelRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_config(&self, guild_id: String) -> Result<LevelConfig, String> {
        self.repo.get_level_config(guild_id).await
    }

    pub async fn get_leaderboard(&self, guild_id: String) -> Result<Vec<UserLevel>, String> {
        self.repo.get_level_leaderboard(guild_id).await
    }

    pub async fn get_rewards(&self, guild_id: String) -> Result<Vec<LevelReward>, String> {
        self.repo.get_level_rewards(guild_id).await
    }
}
