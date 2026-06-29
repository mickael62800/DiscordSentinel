//! Adapter Postgres du port `ExportRepository`. Execute les SELECT
//! d'export et map vers les DTOs purs du port (sans sqlx::FromRow exposÃ©).

use crate::adapters::outbound::postgres::pg_err_ctx;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::system::export_repository::{
    AuditLogExport, ExportRepository, InfractionExport, ModerationActionExport,
};

pub struct PgExportRepository {
    pool: PgPool,
}

impl PgExportRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct InfractionRow {
    id: Uuid,
    guild_id: String,
    channel_id: String,
    user_id: String,
    username: String,
    message_id: String,
    content: String,
    score: f64,
    action: String,
    reason: String,
    duration: Option<i64>,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct AuditLogRow {
    id: Uuid,
    guild_id: String,
    event_type: String,
    actor_id: Option<String>,
    actor_name: Option<String>,
    target_id: Option<String>,
    target_name: Option<String>,
    channel_id: Option<String>,
    channel_name: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct ModerationActionRow {
    id: Uuid,
    guild_id: String,
    moderator_id: String,
    moderator_name: String,
    target_id: String,
    target_name: String,
    action_type: String,
    reason: String,
    duration: Option<i64>,
    created_at: DateTime<Utc>,
}

#[async_trait]
impl ExportRepository for PgExportRepository {
    async fn fetch_infractions(
        &self,
        guild_id: &str,
        max_rows: i64,
    ) -> Result<Vec<InfractionExport>, DomainError> {
        let rows: Vec<InfractionRow> = sqlx::query_as(
            "SELECT id, guild_id, channel_id, user_id, username, message_id, content, \
                    score, action, reason, duration, created_at \
             FROM infractions WHERE guild_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(max_rows)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| pg_err_ctx("query infractions", e))?;
        Ok(rows
            .into_iter()
            .map(|r| InfractionExport {
                id: r.id,
                guild_id: r.guild_id,
                channel_id: r.channel_id,
                user_id: r.user_id,
                username: r.username,
                message_id: r.message_id,
                content: r.content,
                score: r.score,
                action: r.action,
                reason: r.reason,
                duration: r.duration,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn fetch_audit_logs(
        &self,
        guild_id: &str,
        max_rows: i64,
    ) -> Result<Vec<AuditLogExport>, DomainError> {
        let rows: Vec<AuditLogRow> = sqlx::query_as(
            "SELECT id, guild_id, event_type, actor_id, actor_name, target_id, target_name, \
                    channel_id, channel_name, created_at \
             FROM audit_logs WHERE guild_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(max_rows)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| pg_err_ctx("query audit_logs", e))?;
        Ok(rows
            .into_iter()
            .map(|r| AuditLogExport {
                id: r.id,
                guild_id: r.guild_id,
                event_type: r.event_type,
                actor_id: r.actor_id,
                actor_name: r.actor_name,
                target_id: r.target_id,
                target_name: r.target_name,
                channel_id: r.channel_id,
                channel_name: r.channel_name,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn fetch_moderation_actions(
        &self,
        guild_id: &str,
        max_rows: i64,
    ) -> Result<Vec<ModerationActionExport>, DomainError> {
        let rows: Vec<ModerationActionRow> = sqlx::query_as(
            "SELECT id, guild_id, moderator_id, moderator_name, target_id, target_name, \
                    action_type, reason, duration, created_at \
             FROM moderation_actions WHERE guild_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(max_rows)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| pg_err_ctx("query moderation_actions", e))?;
        Ok(rows
            .into_iter()
            .map(|r| ModerationActionExport {
                id: r.id,
                guild_id: r.guild_id,
                moderator_id: r.moderator_id,
                moderator_name: r.moderator_name,
                target_id: r.target_id,
                target_name: r.target_name,
                action_type: r.action_type,
                reason: r.reason,
                duration: r.duration,
                created_at: r.created_at,
            })
            .collect())
    }
}
