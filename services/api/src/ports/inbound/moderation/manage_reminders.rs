use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::moderation::action::sanction_reminder::SanctionReminder;
use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::GuildId;

pub struct CreateReminderCommand {
    pub guild_id: GuildId,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub action_id: Uuid,
    pub duration_secs: u64,
    pub remind_before_secs: u64,
}

#[async_trait]
pub trait ManageRemindersUseCase: Send + Sync {
    async fn create_reminder(&self, cmd: CreateReminderCommand) -> Result<SanctionReminder, DomainError>;
    async fn get_pending_reminders(&self) -> Result<Vec<SanctionReminder>, DomainError>;
    async fn mark_sent(&self, reminder_id: Uuid) -> Result<(), DomainError>;
    async fn cancel_for_action(&self, action_id: Uuid) -> Result<(), DomainError>;
    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<SanctionReminder>, DomainError>;
}
