use async_trait::async_trait;

use sentinel_core::domain::entities::system::log_entry::LogEntry;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait LogRepository: Send + Sync {
    async fn save(&self, entry: &LogEntry) -> Result<(), DomainError>;
    async fn find_all(&self, limit: i64) -> Result<Vec<LogEntry>, DomainError>;
    async fn find_filtered(
        &self,
        category: Option<&str>,
        level: Option<&str>,
        limit: i64,
    ) -> Result<Vec<LogEntry>, DomainError>;
    async fn delete_by_category(&self, category: &str) -> Result<u64, DomainError>;
    async fn delete_older_than_days(&self, days: i32) -> Result<u64, DomainError>;
}
