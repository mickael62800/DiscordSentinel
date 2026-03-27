use std::sync::Arc;

use crate::domain::entities::{UserDossier, WatchedUser};
use crate::domain::ports::WatchedUsersRepository;

pub struct WatchedUsersService {
    repo: Arc<dyn WatchedUsersRepository>,
}

impl WatchedUsersService {
    pub fn new(repo: Arc<dyn WatchedUsersRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_watched_users(&self, guild_id: Option<String>) -> Result<Vec<WatchedUser>, String> {
        self.repo.get_watched_users(guild_id).await
    }

    pub async fn get_user_dossier(&self, guild_id: String, user_id: String) -> Result<UserDossier, String> {
        self.repo.get_user_dossier(guild_id, user_id).await
    }
}
