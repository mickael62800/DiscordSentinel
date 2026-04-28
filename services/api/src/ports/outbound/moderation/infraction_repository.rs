use async_trait::async_trait;

use crate::domain::entities::Infraction;
use crate::domain::errors::DomainError;
use crate::ports::inbound::InfractionFilters;

#[async_trait]
pub trait InfractionRepository: Send + Sync {
    async fn save(&self, infraction: &Infraction) -> Result<(), DomainError>;
    async fn find_by_guild(
        &self,
        guild_id: &str,
        filters: &InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError>;

    async fn find_all(&self, limit: i64, offset: i64) -> Result<Vec<Infraction>, DomainError>;
    async fn count_today(&self) -> Result<u64, DomainError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Infraction>, DomainError>;
    async fn delete_by_id(&self, id: &str) -> Result<bool, DomainError>;
    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError>;
}
