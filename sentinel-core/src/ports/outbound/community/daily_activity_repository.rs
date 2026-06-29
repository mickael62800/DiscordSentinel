use async_trait::async_trait;

use crate::domain::entities::community::daily_activity::DailyActivity;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait DailyActivityRepository: Send + Sync {
    async fn get_activity(
        &self,
        guild_id: Option<&str>,
        days: i32,
    ) -> Result<Vec<DailyActivity>, DomainError>;
    #[allow(dead_code)]
    async fn record_daily_snapshot(&self, guild_id: &str) -> Result<(), DomainError>;
}
