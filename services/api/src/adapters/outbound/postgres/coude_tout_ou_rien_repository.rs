//! Impl Postgres de `CoudeToutOuRienRepository` (cf. COUPE_AMELIORATIONS 6.1).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::{ToutOuRienLogEntry, ToutOuRienLogOutcome, ToutOuRienUserStats};
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

    async fn user_stats(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<ToutOuRienUserStats, DomainError> {
        // Aggregat unique pour eviter 4 round-trips. COALESCE pour les
        // cas vides (jamais joue / jamais gagne / jamais perdu).
        let row: (i64, i64, i64, i64, i64) = sqlx::query_as(
            r#"SELECT
                   COUNT(*)::BIGINT,
                   COUNT(*) FILTER (WHERE outcome = 'won')::BIGINT,
                   COUNT(*) FILTER (WHERE outcome = 'lost')::BIGINT,
                   COALESCE(MAX(delta) FILTER (WHERE outcome = 'won'), 0)::BIGINT,
                   COALESCE(ABS(MIN(delta)) FILTER (WHERE outcome = 'lost'), 0)::BIGINT
               FROM coude_tout_ou_rien_log
               WHERE guild_id = $1 AND user_id = $2"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(ToutOuRienUserStats {
            attempts: row.0,
            wins: row.1,
            losses: row.2,
            biggest_win: row.3,
            biggest_loss: row.4,
        })
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
