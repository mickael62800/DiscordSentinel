use crate::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::ports::outbound::community::confession_repository::ConfessionRepository;
use sentinel_core::domain::entities::community::confession::{
    Confession, ConfessionConfig, ConfessionReply, ConfessionReport, ReportStatus,
};
use sentinel_core::domain::errors::DomainError;

pub struct PgConfessionRepository {
    pool: PgPool,
}

impl PgConfessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct ConfessionRow {
    id: Uuid,
    guild_id: String,
    public_number: i32,
    author_user_id: String,
    content: String,
    message_id: Option<String>,
    channel_id: Option<String>,
    thread_id: Option<String>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<String>,
    deleted_reason: Option<String>,
    edited_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<ConfessionRow> for Confession {
    fn from(r: ConfessionRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            public_number: r.public_number,
            author_user_id: r.author_user_id,
            content: r.content,
            message_id: r.message_id.into(),
            channel_id: r.channel_id.into(),
            thread_id: r.thread_id,
            deleted_at: r.deleted_at,
            deleted_by: r.deleted_by,
            deleted_reason: r.deleted_reason,
            edited_at: r.edited_at,
            created_at: r.created_at,
        }
    }
}

#[derive(FromRow)]
struct ReplyRow {
    id: Uuid,
    confession_id: Uuid,
    public_number: i32,
    author_user_id: String,
    content: String,
    is_anonymous: bool,
    message_id: Option<String>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<String>,
    edited_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<ReplyRow> for ConfessionReply {
    fn from(r: ReplyRow) -> Self {
        Self {
            id: r.id,
            confession_id: r.confession_id,
            public_number: r.public_number,
            author_user_id: r.author_user_id,
            content: r.content,
            is_anonymous: r.is_anonymous,
            message_id: r.message_id.into(),
            deleted_at: r.deleted_at,
            deleted_by: r.deleted_by,
            edited_at: r.edited_at,
            created_at: r.created_at,
        }
    }
}

#[derive(FromRow)]
struct ReportRow {
    id: Uuid,
    guild_id: String,
    confession_id: Option<Uuid>,
    reply_id: Option<Uuid>,
    reporter_user_id: String,
    reason: String,
    status: String,
    resolved_by: Option<String>,
    resolved_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<ReportRow> for ConfessionReport {
    fn from(r: ReportRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            confession_id: r.confession_id,
            reply_id: r.reply_id,
            reporter_user_id: r.reporter_user_id,
            reason: r.reason,
            status: ReportStatus::from_str(&r.status).unwrap_or(ReportStatus::Pending),
            resolved_by: r.resolved_by,
            resolved_at: r.resolved_at,
            created_at: r.created_at,
        }
    }
}

#[derive(FromRow)]
struct ConfigRow {
    guild_id: String,
    enabled: bool,
    channel_id: Option<String>,
    panel_message_id: Option<String>,
    cooldown_secs: i32,
    max_per_day: i32,
    min_chars: i32,
    max_chars: i32,
    automod_enabled: bool,
    banned_user_ids: serde_json::Value,
    updated_at: DateTime<Utc>,
}

impl From<ConfigRow> for ConfessionConfig {
    fn from(r: ConfigRow) -> Self {
        let banned: Vec<String> = serde_json::from_value(r.banned_user_ids).unwrap_or_default();
        Self {
            guild_id: r.guild_id.into(),
            enabled: r.enabled,
            channel_id: r.channel_id.into(),
            panel_message_id: r.panel_message_id,
            cooldown_secs: r.cooldown_secs,
            max_per_day: r.max_per_day,
            min_chars: r.min_chars,
            max_chars: r.max_chars,
            automod_enabled: r.automod_enabled,
            banned_user_ids: banned,
            updated_at: r.updated_at,
        }
    }
}

const SELECT_CONFESSION: &str = "SELECT id, guild_id, public_number, author_user_id, content, \
    message_id, channel_id, thread_id, deleted_at, deleted_by, deleted_reason, edited_at, \
    created_at FROM confessions";

const SELECT_REPLY: &str = "SELECT id, confession_id, public_number, author_user_id, content, \
    is_anonymous, message_id, deleted_at, deleted_by, edited_at, created_at \
    FROM confession_replies";

const SELECT_REPORT: &str = "SELECT id, guild_id, confession_id, reply_id, reporter_user_id, \
    reason, status, resolved_by, resolved_at, created_at FROM confession_reports";

const SELECT_CONFIG: &str = "SELECT guild_id, enabled, channel_id, panel_message_id, \
    cooldown_secs, max_per_day, min_chars, max_chars, automod_enabled, banned_user_ids, \
    updated_at FROM confession_config";

#[async_trait]
impl ConfessionRepository for PgConfessionRepository {
    async fn next_public_number(&self, guild_id: &str) -> Result<i32, DomainError> {
        // Atomique : INSERT ... ON CONFLICT DO UPDATE RETURNING new value
        let row: (i32,) = sqlx::query_as(
            r#"INSERT INTO confession_counters (guild_id, last_number, updated_at)
               VALUES ($1, 1, NOW())
               ON CONFLICT (guild_id) DO UPDATE SET
                 last_number = confession_counters.last_number + 1,
                 updated_at = NOW()
               RETURNING last_number"#,
        )
        .bind(guild_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.0)
    }

    async fn create_confession(&self, c: &Confession) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO confessions
                (id, guild_id, public_number, author_user_id, content,
                 message_id, channel_id, thread_id, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        )
        .bind(c.id)
        .bind(c.guild_id.as_str())
        .bind(c.public_number)
        .bind(&c.author_user_id)
        .bind(&c.content)
        .bind(c.message_id.as_deref())
        .bind(c.channel_id.as_deref())
        .bind(&c.thread_id)
        .bind(c.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn update_confession_message_refs(
        &self,
        id: Uuid,
        message_id: &str,
        channel_id: &str,
        thread_id: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE confessions SET message_id = $2, channel_id = $3, thread_id = $4 WHERE id = $1",
        )
        .bind(id)
        .bind(message_id)
        .bind(channel_id)
        .bind(thread_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn edit_confession_content(&self, id: Uuid, content: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE confessions SET content = $2, edited_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(content)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn soft_delete_confession(
        &self,
        id: Uuid,
        deleted_by: &str,
        reason: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE confessions SET deleted_at = NOW(), deleted_by = $2, deleted_reason = $3 \
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(deleted_by)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn get_confession(&self, id: Uuid) -> Result<Option<Confession>, DomainError> {
        let q = format!("{} WHERE id = $1", SELECT_CONFESSION);
        let row = sqlx::query_as::<_, ConfessionRow>(&q)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Confession::from))
    }

    async fn get_by_message_id(&self, message_id: &str) -> Result<Option<Confession>, DomainError> {
        let q = format!("{} WHERE message_id = $1", SELECT_CONFESSION);
        let row = sqlx::query_as::<_, ConfessionRow>(&q)
            .bind(message_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Confession::from))
    }

    async fn get_by_public_number(
        &self,
        guild_id: &str,
        public_number: i32,
    ) -> Result<Option<Confession>, DomainError> {
        let q = format!(
            "{} WHERE guild_id = $1 AND public_number = $2",
            SELECT_CONFESSION
        );
        let row = sqlx::query_as::<_, ConfessionRow>(&q)
            .bind(guild_id)
            .bind(public_number)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Confession::from))
    }

    async fn list_by_guild(
        &self,
        guild_id: &str,
        limit: i64,
        include_deleted: bool,
    ) -> Result<Vec<Confession>, DomainError> {
        let q = if include_deleted {
            format!(
                "{} WHERE guild_id = $1 ORDER BY public_number DESC LIMIT $2",
                SELECT_CONFESSION
            )
        } else {
            format!(
                "{} WHERE guild_id = $1 AND deleted_at IS NULL ORDER BY public_number DESC LIMIT $2",
                SELECT_CONFESSION
            )
        };
        let rows = sqlx::query_as::<_, ConfessionRow>(&q)
            .bind(guild_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(Confession::from).collect())
    }

    async fn count_recent_by_author(
        &self,
        guild_id: &str,
        author_user_id: &str,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM confessions \
             WHERE guild_id = $1 AND author_user_id = $2 AND created_at >= $3",
        )
        .bind(guild_id)
        .bind(author_user_id)
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(count.0)
    }

    async fn next_reply_public_number(&self, confession_id: Uuid) -> Result<i32, DomainError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COALESCE(MAX(public_number), 0) FROM confession_replies WHERE confession_id = $1",
        )
        .bind(confession_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok((count.0 as i32) + 1)
    }

    async fn create_reply(&self, r: &ConfessionReply) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO confession_replies
                (id, confession_id, public_number, author_user_id, content,
                 is_anonymous, message_id, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(r.id)
        .bind(r.confession_id)
        .bind(r.public_number)
        .bind(&r.author_user_id)
        .bind(&r.content)
        .bind(r.is_anonymous)
        .bind(r.message_id.as_deref())
        .bind(r.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn update_reply_message_id(&self, id: Uuid, message_id: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE confession_replies SET message_id = $2 WHERE id = $1")
            .bind(id)
            .bind(message_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn soft_delete_reply(&self, id: Uuid, deleted_by: &str) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE confession_replies SET deleted_at = NOW(), deleted_by = $2 \
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(deleted_by)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_replies(&self, confession_id: Uuid) -> Result<Vec<ConfessionReply>, DomainError> {
        let q = format!(
            "{} WHERE confession_id = $1 ORDER BY public_number ASC",
            SELECT_REPLY
        );
        let rows = sqlx::query_as::<_, ReplyRow>(&q)
            .bind(confession_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(ConfessionReply::from).collect())
    }

    async fn get_reply(&self, id: Uuid) -> Result<Option<ConfessionReply>, DomainError> {
        let q = format!("{} WHERE id = $1", SELECT_REPLY);
        let row = sqlx::query_as::<_, ReplyRow>(&q)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(ConfessionReply::from))
    }

    async fn create_report(&self, r: &ConfessionReport) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO confession_reports
                (id, guild_id, confession_id, reply_id, reporter_user_id, reason, status, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(r.id)
        .bind(r.guild_id.as_str())
        .bind(r.confession_id)
        .bind(r.reply_id)
        .bind(&r.reporter_user_id)
        .bind(&r.reason)
        .bind(r.status.as_str())
        .bind(r.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_reports(
        &self,
        guild_id: &str,
        status: Option<ReportStatus>,
        limit: i64,
    ) -> Result<Vec<ConfessionReport>, DomainError> {
        let rows = if let Some(s) = status {
            let q = format!(
                "{} WHERE guild_id = $1 AND status = $2 ORDER BY created_at DESC LIMIT $3",
                SELECT_REPORT
            );
            sqlx::query_as::<_, ReportRow>(&q)
                .bind(guild_id)
                .bind(s.as_str())
                .bind(limit)
                .fetch_all(&self.pool)
                .await
        } else {
            let q = format!(
                "{} WHERE guild_id = $1 ORDER BY created_at DESC LIMIT $2",
                SELECT_REPORT
            );
            sqlx::query_as::<_, ReportRow>(&q)
                .bind(guild_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
        }
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(ConfessionReport::from).collect())
    }

    async fn resolve_report(
        &self,
        id: Uuid,
        status: ReportStatus,
        resolved_by: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE confession_reports SET status = $2, resolved_by = $3, resolved_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(status.as_str())
        .bind(resolved_by)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn get_config(&self, guild_id: &str) -> Result<Option<ConfessionConfig>, DomainError> {
        let q = format!("{} WHERE guild_id = $1", SELECT_CONFIG);
        let row = sqlx::query_as::<_, ConfigRow>(&q)
            .bind(guild_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(ConfessionConfig::from))
    }

    async fn upsert_config(&self, c: &ConfessionConfig) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO confession_config
                (guild_id, enabled, channel_id, panel_message_id, cooldown_secs,
                 max_per_day, min_chars, max_chars, automod_enabled, banned_user_ids, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
                ON CONFLICT (guild_id) DO UPDATE SET
                    enabled = $2, channel_id = $3, panel_message_id = $4,
                    cooldown_secs = $5, max_per_day = $6, min_chars = $7, max_chars = $8,
                    automod_enabled = $9, banned_user_ids = $10, updated_at = NOW()"#,
        )
        .bind(c.guild_id.as_str())
        .bind(c.enabled)
        .bind(c.channel_id.as_deref())
        .bind(&c.panel_message_id)
        .bind(c.cooldown_secs)
        .bind(c.max_per_day)
        .bind(c.min_chars)
        .bind(c.max_chars)
        .bind(c.automod_enabled)
        .bind(serde_json::to_value(&c.banned_user_ids).unwrap_or_default())
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
