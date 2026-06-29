use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::super::pg_err_ctx;
use crate::ports::outbound::system::admin_rotation_repository::AdminRotationRepository;
use sentinel_core::domain::entities::system::admin_rotation::{RotationState, ServedEntry};
use sentinel_core::domain::errors::DomainError;

const TBL: &str = "admin_rotation";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

#[derive(sqlx::FromRow)]
struct Row {
    guild_id: String,
    state: String,
    current_admin_id: Option<String>,
    current_admin_since: Option<DateTime<Utc>>,
    period_start: Option<DateTime<Utc>>,
    next_rotation_at: Option<DateTime<Utc>>,
    candidate_id: Option<String>,
    candidate_offered_at: Option<DateTime<Utc>>,
    asked_this_round: serde_json::Value,
}

impl From<Row> for RotationState {
    fn from(r: Row) -> Self {
        let asked = r
            .asked_this_round
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        RotationState {
            guild_id: r.guild_id,
            state: r.state,
            current_admin_id: r.current_admin_id,
            current_admin_since: r.current_admin_since,
            period_start: r.period_start,
            next_rotation_at: r.next_rotation_at,
            candidate_id: r.candidate_id,
            candidate_offered_at: r.candidate_offered_at,
            asked_this_round: asked,
        }
    }
}

pub struct PgAdminRotationRepository {
    pool: PgPool,
}

impl PgAdminRotationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AdminRotationRepository for PgAdminRotationRepository {
    async fn get(&self, guild_id: &str) -> Result<Option<RotationState>, DomainError> {
        let row: Option<Row> = sqlx::query_as("SELECT * FROM admin_rotation WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn upsert(&self, s: &RotationState) -> Result<(), DomainError> {
        let asked = serde_json::Value::from(s.asked_this_round.clone());
        sqlx::query(
            "INSERT INTO admin_rotation \
                (guild_id, state, current_admin_id, current_admin_since, period_start, \
                 next_rotation_at, candidate_id, candidate_offered_at, asked_this_round, updated_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9, NOW()) \
             ON CONFLICT (guild_id) DO UPDATE SET \
                state = EXCLUDED.state, current_admin_id = EXCLUDED.current_admin_id, \
                current_admin_since = EXCLUDED.current_admin_since, period_start = EXCLUDED.period_start, \
                next_rotation_at = EXCLUDED.next_rotation_at, candidate_id = EXCLUDED.candidate_id, \
                candidate_offered_at = EXCLUDED.candidate_offered_at, \
                asked_this_round = EXCLUDED.asked_this_round, updated_at = NOW()",
        )
        .bind(&s.guild_id)
        .bind(&s.state)
        .bind(&s.current_admin_id)
        .bind(s.current_admin_since)
        .bind(s.period_start)
        .bind(s.next_rotation_at)
        .bind(&s.candidate_id)
        .bind(s.candidate_offered_at)
        .bind(&asked)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn record_served(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("INSERT INTO admin_rotation_history (guild_id, user_id) VALUES ($1,$2)")
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn served_entries(&self, guild_id: &str) -> Result<Vec<ServedEntry>, DomainError> {
        #[derive(sqlx::FromRow)]
        struct E {
            user_id: String,
            served_at: DateTime<Utc>,
        }
        let rows: Vec<E> = sqlx::query_as(
            "SELECT user_id, MAX(served_at) AS served_at FROM admin_rotation_history \
             WHERE guild_id = $1 GROUP BY user_id ORDER BY served_at ASC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|e| ServedEntry {
                user_id: e.user_id,
                served_at: e.served_at,
            })
            .collect())
    }
}
