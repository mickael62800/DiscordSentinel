//! Impl Postgres de `CoudeToutOuRienRepository` (cf. COUPE_AMELIORATIONS 6.1).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::{ToutOuRienLogEntry, ToutOuRienLogOutcome};
use crate::domain::errors::DomainError;
use crate::ports::outbound::CoudeToutOuRienRepository;

fn pg_err(e: sqlx::Error) -> DomainError {
    DomainError::Internal(format!("coude_tout_ou_rien_log pg: {e}"))
}

pub struct PgCoudeToutOuRienRepository {
    pool: PgPool,
}

impl PgCoudeToutOuRienRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: String,
    user_id: String,
    username: String,
    mise: i64,
    outcome: String,
    delta: i64,
    created_at: DateTime<Utc>,
}

impl TryFrom<Row> for ToutOuRienLogEntry {
    type Error = DomainError;
    fn try_from(r: Row) -> Result<Self, DomainError> {
        let outcome = ToutOuRienLogOutcome::from_db_str(&r.outcome)
            .ok_or_else(|| DomainError::Internal(format!("outcome inconnu : {}", r.outcome)))?;
        Ok(Self {
            id: r.id,
            guild_id: r.guild_id,
            user_id: r.user_id,
            username: r.username,
            mise: r.mise,
            outcome,
            delta: r.delta,
            created_at: r.created_at,
        })
    }
}

#[async_trait]
impl CoudeToutOuRienRepository for PgCoudeToutOuRienRepository {
    async fn record(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        mise: i64,
        outcome: ToutOuRienLogOutcome,
        delta: i64,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO coude_tout_ou_rien_log
                   (guild_id, user_id, username, mise, outcome, delta)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(username)
        .bind(mise)
        .bind(outcome.as_db_str())
        .bind(delta)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn memorial(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<ToutOuRienLogEntry>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            r#"SELECT id, guild_id, user_id, username, mise, outcome, delta, created_at
               FROM coude_tout_ou_rien_log
               WHERE guild_id = $1 AND outcome = 'lost'
               ORDER BY delta ASC
               LIMIT $2"#,
        )
        .bind(guild_id)
        .bind(limit.max(1).min(50))
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        rows.into_iter()
            .map(ToutOuRienLogEntry::try_from)
            .collect::<Result<Vec<_>, _>>()
    }
}
