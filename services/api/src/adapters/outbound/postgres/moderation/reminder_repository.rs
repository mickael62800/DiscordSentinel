use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::moderation::action::sanction_reminder::SanctionReminder;
use crate::domain::errors::DomainError;
use crate::ports::outbound::moderation::reminder_repository::ReminderRepository;
use crate::domain::entities::system::discord_ids::GuildId;

pub struct PgReminderRepository {
    pool: PgPool,
}

impl PgReminderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ReminderRow {
    id: Uuid,
    guild_id: GuildId,
    moderator_id: String,
    moderator_name: String,
    target_id: String,
    target_name: String,
    action_type: String,
    reason: String,
    action_id: Uuid,
    remind_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    status: String,
    created_at: DateTime<Utc>,
}

impl From<ReminderRow> for SanctionReminder {
    fn from(r: ReminderRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            moderator_id: r.moderator_id,
            moderator_name: r.moderator_name,
            target_id: r.target_id,
            target_name: r.target_name,
            action_type: r.action_type,
            reason: r.reason,
            action_id: r.action_id,
            remind_at: r.remind_at,
            expires_at: r.expires_at,
            status: r.status,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl ReminderRepository for PgReminderRepository {
    async fn save(&self, r: &SanctionReminder) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO sanction_reminders (id, guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, action_id, remind_at, expires_at, status, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"
        )
        .bind(r.id)
        .bind(&r.guild_id)
        .bind(&r.moderator_id)
        .bind(&r.moderator_name)
        .bind(&r.target_id)
        .bind(&r.target_name)
        .bind(&r.action_type)
        .bind(&r.reason)
        .bind(r.action_id)
        .bind(r.remind_at)
        .bind(r.expires_at)
        .bind(&r.status)
        .bind(r.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("save_reminder: {e}")))?;
        Ok(())
    }

    async fn find_pending(&self) -> Result<Vec<SanctionReminder>, DomainError> {
        let rows = sqlx::query_as::<_, ReminderRow>(
            "SELECT id, guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, action_id, remind_at, expires_at, status, created_at
             FROM sanction_reminders
             WHERE status = 'pending' AND remind_at <= NOW()
             ORDER BY remind_at ASC
             LIMIT 100"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("find_pending_reminders: {e}")))?;

        Ok(rows.into_iter().map(SanctionReminder::from).collect())
    }

    async fn mark_sent(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE sanction_reminders SET status = 'sent' WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(format!("mark_sent: {e}")))?;
        Ok(())
    }

    async fn cancel_for_action(&self, action_id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE sanction_reminders SET status = 'cancelled' WHERE action_id = $1 AND status = 'pending'")
            .bind(action_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(format!("cancel_reminders: {e}")))?;
        Ok(())
    }

    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<SanctionReminder>, DomainError> {
        let rows = sqlx::query_as::<_, ReminderRow>(
            "SELECT id, guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, action_id, remind_at, expires_at, status, created_at
             FROM sanction_reminders WHERE guild_id = $1 ORDER BY remind_at DESC LIMIT 50"
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("find_reminders_by_guild: {e}")))?;

        Ok(rows.into_iter().map(SanctionReminder::from).collect())
    }
}
