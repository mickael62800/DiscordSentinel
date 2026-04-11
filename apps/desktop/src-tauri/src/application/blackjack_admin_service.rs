use std::sync::Arc;

use crate::domain::entities::BlackjackGame;
use crate::domain::ports::BlackjackAdminRepository;

pub struct BlackjackAdminService {
    repo: Arc<dyn BlackjackAdminRepository>,
}

impl BlackjackAdminService {
    pub fn new(repo: Arc<dyn BlackjackAdminRepository>) -> Self {
        Self { repo }
    }

    pub async fn list_games(&self, guild_id: String, status: Option<String>) -> Result<Vec<BlackjackGame>, String> {
        self.repo.list_games(guild_id, status).await
    }

    pub async fn cancel_game(&self, game_id: String) -> Result<(), String> {
        self.repo.cancel_game(game_id).await
    }
}
