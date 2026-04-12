use async_trait::async_trait;

use crate::domain::entities::ModerationAction;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ModerationRepository: Send + Sync {
    async fn save(&self, action: &ModerationAction) -> Result<(), DomainError>;
    async fn find_by_target(&self, guild_id: &str, target_id: &str) -> Result<Vec<ModerationAction>, DomainError>;
    async fn find_bans(&self, guild_id: Option<&str>, limit: i64, offset: i64) -> Result<Vec<ModerationAction>, DomainError>;
    async fn delete_bans_for_user(&self, guild_id: &str, target_id: &str) -> Result<(), DomainError>;
    async fn delete_action(&self, id: uuid::Uuid) -> Result<bool, DomainError>;
}
