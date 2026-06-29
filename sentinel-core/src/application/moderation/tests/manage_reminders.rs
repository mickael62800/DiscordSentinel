//! Tests unitaires du ManageRemindersService.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::application::moderation::manage_reminders_service::ManageRemindersService;
use crate::domain::entities::moderation::action::sanction_reminder::*;
use crate::domain::errors::DomainError;
use crate::ports::inbound::moderation::manage_reminders::*;
use crate::ports::outbound::moderation::reminder_repository::ReminderRepository;

// ══════════════════════════════════════════════════════════
// In-memory Reminder Repository
// ══════════════════════════════════════════════════════════

struct InMemoryReminderRepo {
    reminders: Mutex<Vec<SanctionReminder>>,
}

impl InMemoryReminderRepo {
    fn new() -> Self {
        Self {
            reminders: Mutex::new(vec![]),
        }
    }
}

#[async_trait]
impl ReminderRepository for InMemoryReminderRepo {
    async fn save(&self, r: &SanctionReminder) -> Result<(), DomainError> {
        self.reminders.lock().await.push(r.clone());
        Ok(())
    }

    async fn find_pending(&self) -> Result<Vec<SanctionReminder>, DomainError> {
        let now = Utc::now();
        let reminders = self.reminders.lock().await;
        Ok(reminders
            .iter()
            .filter(|r| r.status == "pending" && r.remind_at <= now)
            .cloned()
            .collect())
    }

    async fn mark_sent(&self, id: Uuid) -> Result<(), DomainError> {
        let mut reminders = self.reminders.lock().await;
        if let Some(r) = reminders.iter_mut().find(|r| r.id == id) {
            r.status = "sent".into();
        }
        Ok(())
    }

    async fn cancel_for_action(&self, action_id: Uuid) -> Result<(), DomainError> {
        let mut reminders = self.reminders.lock().await;
        for r in reminders.iter_mut() {
            if r.action_id == action_id && r.status == "pending" {
                r.status = "cancelled".into();
            }
        }
        Ok(())
    }

    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<SanctionReminder>, DomainError> {
        let reminders = self.reminders.lock().await;
        Ok(reminders
            .iter()
            .filter(|r| r.guild_id.as_str() == guild_id)
            .cloned()
            .collect())
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_service() -> ManageRemindersService {
    let repo = Arc::new(InMemoryReminderRepo::new());
    ManageRemindersService::new(repo as Arc<dyn ReminderRepository>)
}

fn make_cmd(duration_secs: u64, remind_before: u64) -> CreateReminderCommand {
    CreateReminderCommand {
        guild_id: "g1".into(),
        moderator_id: "mod1".into(),
        moderator_name: "Bob".into(),
        target_id: "u1".into(),
        target_name: "Alice".into(),
        action_type: "mute_temp".into(),
        reason: "Spam".into(),
        action_id: Uuid::new_v4(),
        duration_secs,
        remind_before_secs: remind_before,
    }
}

// ══════════════════════════════════════════════════════════
// Tests — create_reminder
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn create_reminder_success() {
    let svc = build_service();
    let result = svc.create_reminder(make_cmd(7200, 3600)).await;
    assert!(result.is_ok());
    let r = result.unwrap();
    assert_eq!(r.guild_id.as_str(), "g1");
    assert_eq!(r.moderator_id, "mod1");
    assert_eq!(r.status, "pending");
    assert!(r.remind_at < r.expires_at);
}

#[tokio::test]
async fn create_reminder_too_short_duration_rejected() {
    let svc = build_service();
    // 30min duration, 1h remind_before → impossible
    let result = svc.create_reminder(make_cmd(1800, 3600)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn create_reminder_remind_at_is_before_expires_at() {
    let svc = build_service();
    let r = svc.create_reminder(make_cmd(7200, 3600)).await.unwrap();
    // remind_at should be ~1h before expires_at
    let diff = r.expires_at.signed_duration_since(r.remind_at);
    assert!((diff.num_seconds() - 3600).abs() < 5); // within 5s tolerance
}

// ══════════════════════════════════════════════════════════
// Tests — mark_sent
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn mark_sent_changes_status() {
    let svc = build_service();
    let r = svc.create_reminder(make_cmd(7200, 3600)).await.unwrap();
    svc.mark_sent(r.id).await.unwrap();

    let pending = svc.get_pending_reminders().await.unwrap();
    assert!(pending.iter().all(|p| p.id != r.id));
}

// ══════════════════════════════════════════════════════════
// Tests — cancel_for_action
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn cancel_for_action_cancels_pending() {
    let svc = build_service();
    let action_id = Uuid::new_v4();
    let mut cmd = make_cmd(7200, 3600);
    cmd.action_id = action_id;
    svc.create_reminder(cmd).await.unwrap();

    svc.cancel_for_action(action_id).await.unwrap();

    let by_guild = svc.list_by_guild("g1").await.unwrap();
    assert_eq!(by_guild[0].status, "cancelled");
}

// ══════════════════════════════════════════════════════════
// Tests — list_by_guild
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn list_by_guild_returns_guild_reminders() {
    let svc = build_service();
    svc.create_reminder(make_cmd(7200, 3600)).await.unwrap();

    let reminders = svc.list_by_guild("g1").await.unwrap();
    assert_eq!(reminders.len(), 1);

    let empty = svc.list_by_guild("other").await.unwrap();
    assert!(empty.is_empty());
}
