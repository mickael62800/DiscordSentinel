use async_trait::async_trait;
use uuid::Uuid;

use sentinel_core::domain::entities::moderation::action::sanction_reminder::SanctionReminder;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait ReminderRepository: Send + Sync {
    async fn save(&self, reminder: &SanctionReminder) -> Result<(), DomainError>;
    async fn find_pending(&self) -> Result<Vec<SanctionReminder>, DomainError>;
    async fn mark_sent(&self, id: Uuid) -> Result<(), DomainError>;
    async fn cancel_for_action(&self, action_id: Uuid) -> Result<(), DomainError>;
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<SanctionReminder>, DomainError>;
}
