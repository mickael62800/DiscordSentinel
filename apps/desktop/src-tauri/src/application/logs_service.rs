use std::sync::Arc;

use crate::domain::entities::LogEntry;
use crate::domain::ports::LogsRepository;

pub struct LogsService {
    repo: Arc<dyn LogsRepository>,
}

impl LogsService {
    pub fn new(repo: Arc<dyn LogsRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_logs(&self, guild_id: Option<String>) -> Result<Vec<LogEntry>, String> {
        self.repo.get_logs(guild_id).await
    }
}
