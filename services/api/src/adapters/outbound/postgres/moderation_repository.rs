use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::ModerationAction;
use crate::domain::errors::DomainError;
use crate::domain::value_objects::ModerationGravity;
use crate::ports::outbound::ModerationRepository;

pub struct PgModerationRepository {
    pool: PgPool,
}

impl PgModerationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ActionRow {
    id: Uuid,
    guild_id: String,
    channel_id: String,
    moderator_id: String,
    moderator_name: String,
    target_id: String,
    target_name: String,
    action_type: String,
    reason: String,
    gravity: Option<ModerationGravity>,
    duration: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ActionRow> for ModerationAction {
    fn from(row: ActionRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id,
            channel_id: row.channel_id,
            moderator_id: row.moderator_id,
            moderator_name: row.moderator_name,
            target_id: row.target_id,
            target_name: row.target_name,
            action_type: row.action_type,
            reason: row.reason,
            gravity: row.gravity,
            duration: row.duration.map(|d| d as u64),
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl ModerationRepository for PgModerationRepository {
    async fn save(&self, action: &ModerationAction) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO moderation_actions (id, guild_id, channel_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, gravity, duration, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(action.id)
        .bind(&action.guild_id)
        .bind(&action.channel_id)
        .bind(&action.moderator_id)
        .bind(&action.moderator_name)
        .bind(&action.target_id)
        .bind(&action.target_name)
        .bind(&action.action_type)
        .bind(&action.reason)
        .bind(action.gravity)
        .bind(action.duration.map(|d| d as i64))
        .bind(action.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn find_by_target(&self, guild_id: &str, target_id: &str) -> Result<Vec<ModerationAction>, DomainError> {
        let rows = sqlx::query_as::<_, ActionRow>(
            "SELECT * FROM moderation_actions WHERE guild_id = $1 AND target_id = $2 ORDER BY created_at DESC LIMIT 50",
        )
        .bind(guild_id)
        .bind(target_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(ModerationAction::from).collect())
    }

    async fn find_bans(&self, guild_id: Option<&str>, limit: i64, offset: i64) -> Result<Vec<ModerationAction>, DomainError> {
        let rows = match guild_id {
            Some(gid) => {
                sqlx::query_as::<_, ActionRow>(
                    "SELECT * FROM moderation_actions WHERE action_type LIKE 'ban%' AND guild_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
                )
                .bind(gid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, ActionRow>(
                    "SELECT * FROM moderation_actions WHERE action_type LIKE 'ban%' ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(ModerationAction::from).collect())
    }

    async fn delete_bans_for_user(&self, guild_id: &str, target_id: &str) -> Result<(), DomainError> {
        sqlx::query(
            "DELETE FROM moderation_actions WHERE guild_id = $1 AND target_id = $2 AND action_type LIKE 'ban%'",
        )
        .bind(guild_id)
        .bind(target_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn delete_action(&self, id: uuid::Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query("DELETE FROM moderation_actions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(result.rows_affected() > 0)
    }
}
