use std::sync::Arc;

use crate::domain::entities::{CoudeCombat, CoudePlayer};
use crate::domain::ports::CoudeRepository;

pub struct CoudeService {
    repo: Arc<dyn CoudeRepository>,
}

impl CoudeService {
    pub fn new(repo: Arc<dyn CoudeRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_combats(&self, guild_id: String, status: Option<String>) -> Result<Vec<CoudeCombat>, String> {
        self.repo.get_combats(guild_id, status).await
    }

    pub async fn get_players(&self, guild_id: String) -> Result<Vec<CoudePlayer>, String> {
        self.repo.get_players(guild_id).await
    }

    pub async fn cancel_combat(&self, combat_id: String) -> Result<(), String> {
        self.repo.cancel_combat(combat_id).await
    }
}
