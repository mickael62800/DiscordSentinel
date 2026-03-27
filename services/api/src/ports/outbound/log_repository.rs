use async_trait::async_trait;

use crate::domain::entities::LogEntry;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait LogRepository: Send + Sync {
    async fn save(&self, entry: &LogEntry) -> Result<(), DomainError>;
    async fn find_all(&self, limit: i64) -> Result<Vec<LogEntry>, DomainError>;
}
