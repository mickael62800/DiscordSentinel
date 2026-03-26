use async_trait::async_trait;

use crate::domain::entities::Rule;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait CachePort: Send + Sync {
    async fn get_rules(&self, guild_id: &str) -> Result<Option<Vec<Rule>>, DomainError>;
    async fn set_rules(&self, guild_id: &str, rules: &[Rule]) -> Result<(), DomainError>;
    async fn invalidate_rules(&self, guild_id: &str) -> Result<(), DomainError>;
}
