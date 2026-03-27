use async_trait::async_trait;

use crate::domain::entities::WatchedUser;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait WatchedUserRepository: Send + Sync {
    async fn find_watched_users(
        &self,
        guild_id: Option<&str>,
    ) -> Result<Vec<WatchedUser>, DomainError>;
}
