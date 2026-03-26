use std::sync::Arc;

use crate::domain::entities::SecurityEvent;
use crate::domain::ports::SecurityRepository;

pub struct SecurityService {
    repo: Arc<dyn SecurityRepository>,
}

impl SecurityService {
    pub fn new(repo: Arc<dyn SecurityRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_events(&self, guild_id: Option<String>) -> Result<Vec<SecurityEvent>, String> {
        self.repo.get_events(guild_id).await
    }
}
