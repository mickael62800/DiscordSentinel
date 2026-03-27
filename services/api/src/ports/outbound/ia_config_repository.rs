use async_trait::async_trait;

use crate::domain::entities::IaConfig;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait IaConfigRepository: Send + Sync {
    async fn get(&self, guild_id: &str) -> Result<Option<IaConfig>, DomainError>;
    async fn save(&self, config: &IaConfig) -> Result<IaConfig, DomainError>;
}
