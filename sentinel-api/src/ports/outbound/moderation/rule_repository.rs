use async_trait::async_trait;
use uuid::Uuid;

use sentinel_core::domain::entities::system::rule::Rule;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait RuleRepository: Send + Sync {
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<Rule>, DomainError>;
    async fn find_all(&self) -> Result<Vec<Rule>, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Rule>, DomainError>;
    async fn save(&self, rule: &Rule) -> Result<Rule, DomainError>;
    async fn toggle(&self, id: Uuid, enabled: bool) -> Result<(), DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}
