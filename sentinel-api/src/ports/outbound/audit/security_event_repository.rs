use async_trait::async_trait;

use sentinel_core::domain::entities::audit::security_event::SecurityEvent;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait SecurityEventRepository: Send + Sync {
    async fn save(&self, event: &SecurityEvent) -> Result<(), DomainError>;
    async fn find_all(&self) -> Result<Vec<SecurityEvent>, DomainError>;
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<SecurityEvent>, DomainError>;
}
