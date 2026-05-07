use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::moderation::action::sanction_reminder::SanctionReminder;
use crate::domain::errors::DomainError;
use crate::ports::inbound::moderation::manage_reminders::CreateReminderCommand;
use crate::ports::inbound::moderation::manage_reminders::ManageRemindersUseCase;
use crate::ports::outbound::moderation::reminder_repository::ReminderRepository;

const DEFAULT_REMIND_BEFORE_SECS: u64 = 3600; // 1 heure avant expiration

pub struct ManageRemindersService {
    repo: Arc<dyn ReminderRepository>,
}

impl ManageRemindersService {
    pub fn new(repo: Arc<dyn ReminderRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageRemindersUseCase for ManageRemindersService {
    async fn create_reminder(&self, cmd: CreateReminderCommand) -> Result<SanctionReminder, DomainError> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(cmd.duration_secs as i64);
        let remind_before = if cmd.remind_before_secs > 0 {
            cmd.remind_before_secs
        } else {
            DEFAULT_REMIND_BEFORE_SECS
        };

        // Ne pas créer de rappel si la durée est trop courte (< remind_before)
        if cmd.duration_secs <= remind_before {
            return Err(DomainError::ValidationError(
                "Duree de la sanction trop courte pour un rappel".into(),
            ));
        }

        let remind_at = expires_at - Duration::seconds(remind_before as i64);

        let reminder = SanctionReminder {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            moderator_id: cmd.moderator_id,
            moderator_name: cmd.moderator_name,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            action_type: cmd.action_type,
            reason: cmd.reason,
            action_id: cmd.action_id,
            remind_at,
            expires_at,
            status: "pending".into(),
            created_at: now,
        };

        self.repo.save(&reminder).await?;
        Ok(reminder)
    }

    async fn get_pending_reminders(&self) -> Result<Vec<SanctionReminder>, DomainError> {
        self.repo.find_pending().await
    }

    async fn mark_sent(&self, reminder_id: Uuid) -> Result<(), DomainError> {
        self.repo.mark_sent(reminder_id).await
    }

    async fn cancel_for_action(&self, action_id: Uuid) -> Result<(), DomainError> {
        self.repo.cancel_for_action(action_id).await
    }

    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<SanctionReminder>, DomainError> {
        self.repo.find_by_guild(guild_id).await
    }
}

#[cfg(test)]
#[path = "tests/manage_reminders.rs"]
mod tests;
