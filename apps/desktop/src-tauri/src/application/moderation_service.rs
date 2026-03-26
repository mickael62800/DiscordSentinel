use std::sync::Arc;

use crate::domain::entities::{ModerationActionRequest, ModerationActionResponse, UserModerationHistory};
use crate::domain::ports::ModerationRepository;

pub struct ModerationService {
    repo: Arc<dyn ModerationRepository>,
}

impl ModerationService {
    pub fn new(repo: Arc<dyn ModerationRepository>) -> Self {
        Self { repo }
    }

    pub async fn log_action(&self, action: ModerationActionRequest) -> Result<ModerationActionResponse, String> {
        self.repo.log_action(action).await
    }

    pub async fn get_history(&self, guild_id: String, user_id: String) -> Result<UserModerationHistory, String> {
        self.repo.get_history(guild_id, user_id).await
    }
}
