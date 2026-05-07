use async_trait::async_trait;

use sentinel_core::domain::entities::system::guild::Guild;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
#[allow(dead_code)]
pub trait GuildRepository: Send + Sync {
    async fn upsert(&self, guild: &Guild) -> Result<(), DomainError>;
    async fn find_all(&self) -> Result<Vec<Guild>, DomainError>;
    async fn find_by_id(&self, guild_id: &str) -> Result<Option<Guild>, DomainError>;
}
