use std::sync::Arc;

use crate::domain::entities::{ConductConfig, ConductPointsLog, UserConductPoints};
use crate::domain::ports::ConductRepository;

pub struct ConductService {
    repo: Arc<dyn ConductRepository>,
}

impl ConductService {
    pub fn new(repo: Arc<dyn ConductRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_config(&self, guild_id: String) -> Result<ConductConfig, String> {
        self.repo.get_config(guild_id).await
    }

    pub async fn get_leaderboard(&self, guild_id: String) -> Result<Vec<UserConductPoints>, String> {
        self.repo.get_leaderboard(guild_id).await
    }

    pub async fn get_points(&self, guild_id: String, user_id: String) -> Result<UserConductPoints, String> {
        self.repo.get_points(guild_id, user_id).await
    }

    pub async fn get_log(&self, guild_id: String, user_id: String) -> Result<Vec<ConductPointsLog>, String> {
        self.repo.get_log(guild_id, user_id).await
    }

    pub async fn adjust_points(&self, guild_id: String, user_id: String, amount: i32, reason: String) -> Result<UserConductPoints, String> {
        self.repo.adjust_points(guild_id, user_id, amount, reason).await
    }
}
