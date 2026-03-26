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

    pub async fn get_logs(&self) -> Result<Vec<LogEntry>, String> {
        self.repo.get_logs().await
    }
}
