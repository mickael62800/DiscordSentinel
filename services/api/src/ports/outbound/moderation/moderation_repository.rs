use async_trait::async_trait;

use crate::domain::entities::moderation::moderation_action::ModerationAction;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ModerationRepository: Send + Sync {
    async fn save(&self, action: &ModerationAction) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<ModerationAction>, DomainError>;
    async fn find_by_target(&self, guild_id: &str, target_id: &str, limit: i64) -> Result<Vec<ModerationAction>, DomainError>;
    async fn find_bans(&self, guild_id: Option<&str>, limit: i64, offset: i64) -> Result<Vec<ModerationAction>, DomainError>;
    /// Liste toutes les actions de moderation (warn, mute, ban, unban, etc.)
    /// pour une guild (ou toutes si guild_id = None). Utilise pour le journal
    /// unifie du panneau admin.
    async fn find_all_for_guild(&self, guild_id: Option<&str>, limit: i64) -> Result<Vec<ModerationAction>, DomainError>;
    async fn delete_bans_for_user(&self, guild_id: &str, target_id: &str) -> Result<(), DomainError>;
    async fn delete_action(&self, id: uuid::Uuid) -> Result<bool, DomainError>;
}
