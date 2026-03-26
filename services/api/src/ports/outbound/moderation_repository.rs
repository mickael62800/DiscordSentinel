use async_trait::async_trait;

use crate::domain::entities::ModerationAction;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ModerationRepository: Send + Sync {
    async fn save(&self, action: &ModerationAction) -> Result<(), DomainError>;
    async fn find_by_target(&self, guild_id: &str, target_id: &str) -> Result<Vec<ModerationAction>, DomainError>;
}
