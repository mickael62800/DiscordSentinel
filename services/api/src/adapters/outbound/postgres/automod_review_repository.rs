use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::pg_err_ctx;
use crate::domain::entities::moderation::automod_review::AutomodReview;
use crate::domain::entities::moderation::automod_review::NewAutomodReview;
use crate::domain::errors::DomainError;
use crate::ports::outbound::moderation::automod_review_repository::AutomodReviewRepository;

const TBL: &str = "automod_reviews";
fn pg_err(e: sqlx::Error) -> DomainError { pg_err_ctx(TBL, e) }

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: String,
    channel_id: String,
    message_id: String,
    user_id: String,
    user_name: String,
    content_preview: String,
    suggested_action: String,
    score: f64,
    reason: String,
    flags: serde_json::Value,
    status: String,
    applied_action: Option<String>,
    resolved_by_id: Option<String>,
    resolved_by_name: Option<String>,
    resolved_source: Option<String>,
    created_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
}

impl From<Row> for AutomodReview {
    fn from(r: Row) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            channel_id: r.channel_id,
            message_id: r.message_id,
            user_id: r.user_id,
            user_name: r.user_name,
            content_preview: r.content_preview,
            suggested_action: r.suggested_action,
            score: r.score,
            reason: r.reason,
            flags: r.flags,
            status: r.status,
            applied_action: r.applied_action,
            resolved_by_id: r.resolved_by_id,
            resolved_by_name: r.resolved_by_name,
            resolved_source: r.resolved_source,
            created_at: r.created_at,
            resolved_at: r.resolved_at,
        }
    }
}

pub struct PgAutomodReviewRepository {
    pool: PgPool,
}

impl PgAutomodReviewRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl AutomodReviewRepository for PgAutomodReviewRepository {
    async fn create(&self, r: NewAutomodReview) -> Result<AutomodReview, DomainError> {
        let row: Row = sqlx::query_as(
            "INSERT INTO automod_reviews \
                (guild_id, channel_id, message_id, user_id, user_name, content_preview, \
                 suggested_action, score, reason, flags) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) \
             RETURNING *",
        )
        .bind(&r.guild_id)
        .bind(&r.channel_id)
        .bind(&r.message_id)
        .bind(&r.user_id)
        .bind(&r.user_name)
        .bind(&r.content_preview)
        .bind(r.suggested_action.as_str())
        .bind(r.score)
        .bind(&r.reason)
        .bind(&r.flags)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn get(&self, id: Uuid) -> Result<Option<AutomodReview>, DomainError> {
        let row: Option<Row> = sqlx::query_as("SELECT * FROM automod_reviews WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn list_pending(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT * FROM automod_reviews \
             WHERE guild_id = $1 AND status = 'pending' \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_recent(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT * FROM automod_reviews \
             WHERE guild_id = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn resolve(
        &self,
        id: Uuid,
        applied_action: &str,
        resolved_by_id: &str,
        resolved_by_name: &str,
        resolved_source: &str,
    ) -> Result<AutomodReview, DomainError> {
        let new_status = if applied_action == "ignore" { "ignored" } else { "applied" };
        let row: Option<Row> = sqlx::query_as(
            "UPDATE automod_reviews SET \
                status = $1, applied_action = $2, resolved_by_id = $3, \
                resolved_by_name = $4, resolved_source = $5, resolved_at = NOW() \
             WHERE id = $6 AND status = 'pending' \
             RETURNING *",
        )
        .bind(new_status)
        .bind(applied_action)
        .bind(resolved_by_id)
        .bind(resolved_by_name)
        .bind(resolved_source)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        match row {
            Some(r) => Ok(r.into()),
            None => {
                // Soit la review n existe pas, soit deja resolue.
                let exists: Option<(String,)> =
                    sqlx::query_as("SELECT status FROM automod_reviews WHERE id = $1")
                        .bind(id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(pg_err)?;
                match exists {
                    None => Err(DomainError::NotFound(format!("review {id} introuvable"))),
                    Some((s,)) => Err(DomainError::Conflict(format!(
                        "review deja resolue (status={s})"
                    ))),
                }
            }
        }
    }
}
