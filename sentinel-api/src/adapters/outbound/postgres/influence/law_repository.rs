//! Impl Postgres de `LawRepository`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::influence::law::{Law, LawStatus};
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::influence::law_repository::LawRepository;

use super::super::pg_err_ctx;

const TBL: &str = "influence_laws";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgLawRepository {
    pool: PgPool,
}

impl PgLawRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: String,
    title: String,
    body: String,
    status: String,
    author_id: Uuid,
    expires_at: Option<DateTime<Utc>>,
    channel_id: Option<String>,
    message_id: Option<String>,
    effect_key: Option<String>,
    effect_value: Option<i64>,
    funding_pour: i64,
    funding_contre: i64,
}

impl TryFrom<Row> for Law {
    type Error = DomainError;
    fn try_from(r: Row) -> Result<Self, DomainError> {
        let status = LawStatus::from_str_lossy(&r.status)
            .ok_or_else(|| DomainError::Internal(format!("statut loi inconnu : {}", r.status)))?;
        Ok(Self {
            id: r.id,
            guild_id: r.guild_id,
            title: r.title,
            body: r.body,
            status,
            author_id: r.author_id,
            closes_at: r.expires_at,
            channel_id: r.channel_id,
            message_id: r.message_id,
            effect_key: r.effect_key,
            effect_value: r.effect_value,
            funding_pour: r.funding_pour,
            funding_contre: r.funding_contre,
        })
    }
}

const COLS: &str = "id, guild_id, title, body, status, author_id, expires_at, \
     channel_id, message_id, effect_key, effect_value, funding_pour, funding_contre";

#[async_trait]
impl LawRepository for PgLawRepository {
    async fn create(
        &self,
        guild_id: &str,
        title: &str,
        body: &str,
        author_id: Uuid,
        closes_at: DateTime<Utc>,
        effect_key: Option<&str>,
        effect_value: Option<i64>,
    ) -> Result<Law, DomainError> {
        let sql = format!(
            "INSERT INTO influence_laws \
             (guild_id, title, body, status, author_id, expires_at, effect_key, effect_value) \
             VALUES ($1, $2, $3, 'vote', $4, $5, $6, $7) RETURNING {COLS}"
        );
        let row: Row = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(title)
            .bind(body)
            .bind(author_id)
            .bind(closes_at)
            .bind(effect_key)
            .bind(effect_value)
            .fetch_one(&self.pool)
            .await
            .map_err(pg_err)?;
        row.try_into()
    }

    async fn get(&self, id: Uuid) -> Result<Option<Law>, DomainError> {
        let sql = format!("SELECT {COLS} FROM influence_laws WHERE id = $1");
        let row: Option<Row> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn set_message(
        &self,
        id: Uuid,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE influence_laws SET channel_id = $2, message_id = $3 WHERE id = $1")
            .bind(id)
            .bind(channel_id)
            .bind(message_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn close(&self, id: Uuid, status: LawStatus) -> Result<bool, DomainError> {
        // Garde `status = 'vote'` : deux clotures concurrentes (worker + trigger
        // HTTP) n'archivent / n'appliquent l'effet qu'une seule fois.
        let r = sqlx::query("UPDATE influence_laws SET status = $2 WHERE id = $1 AND status = 'vote'")
            .bind(id)
            .bind(status.as_str())
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(r.rows_affected() == 1)
    }

    async fn add_funding(
        &self,
        law_id: Uuid,
        pour_delta: i64,
        contre_delta: i64,
    ) -> Result<bool, DomainError> {
        // Financement possible uniquement tant que la loi est en vote.
        let r = sqlx::query(
            "UPDATE influence_laws \
             SET funding_pour = funding_pour + $2, funding_contre = funding_contre + $3 \
             WHERE id = $1 AND status = 'vote'",
        )
        .bind(law_id)
        .bind(pour_delta)
        .bind(contre_delta)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(r.rows_affected() == 1)
    }

    async fn list_due(&self, now: DateTime<Utc>) -> Result<Vec<Law>, DomainError> {
        let sql = format!(
            "SELECT {COLS} FROM influence_laws \
             WHERE status = 'vote' AND expires_at IS NOT NULL AND expires_at <= $1"
        );
        let rows: Vec<Row> = sqlx::query_as(&sql)
            .bind(now)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn list_active(&self, guild_id: &str) -> Result<Vec<Law>, DomainError> {
        let sql = format!(
            "SELECT {COLS} FROM influence_laws \
             WHERE guild_id = $1 AND status = 'vote' \
             ORDER BY expires_at ASC NULLS LAST LIMIT 25"
        );
        let rows: Vec<Row> = sqlx::query_as(&sql)
            .bind(guild_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}
