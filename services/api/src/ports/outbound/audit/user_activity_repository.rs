use async_trait::async_trait;

use crate::domain::entities::UserActivity;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait UserActivityRepository: Send + Sync {
    async fn create(&self, activity: &UserActivity) -> Result<(), DomainError>;
    async fn list(
        &self,
        guild_id: &str,
        user_id: &str,
        event_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserActivity>, DomainError>;
}
