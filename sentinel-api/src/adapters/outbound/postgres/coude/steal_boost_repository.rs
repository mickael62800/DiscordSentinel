//! Impl Postgres de `StealBoostRepository` (Phase 9 Part C).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::ports::outbound::coude::steal_boost_repository::StealBoostRepository;
use sentinel_core::domain::entities::coude::steal::boost::StealBoost;
use sentinel_core::domain::errors::DomainError;

use super::super::pg_err_ctx;
const TBL: &str = "steal_boost";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgStealBoostRepository {
    pool: PgPool,
}

impl PgStealBoostRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct BoostRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    item_key: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl From<BoostRow> for StealBoost {
    fn from(r: BoostRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            user_id: r.user_id.into(),
            item_key: r.item_key,
            expires_at: r.expires_at,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl StealBoostRepository for PgStealBoostRepository {
    async fn list_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<StealBoost>, DomainError> {
        let rows: Vec<BoostRow> = sqlx::query_as(
            r#"SELECT id, guild_id, user_id, item_key, expires_at, created_at
               FROM coude_steal_boosts
               WHERE guild_id = $1 AND user_id = $2 AND expires_at > NOW()
               ORDER BY expires_at DESC"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn upsert(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        days_to_add: i64,
    ) -> Result<DateTime<Utc>, DomainError> {
        let row: (DateTime<Utc>,) = sqlx::query_as(
            r#"INSERT INTO coude_steal_boosts (guild_id, user_id, item_key, expires_at)
               VALUES ($1, $2, $3, NOW() + make_interval(days => $4::int))
               ON CONFLICT (guild_id, user_id, item_key) DO UPDATE
                   SET expires_at = GREATEST(
                           coude_steal_boosts.expires_at,
                           NOW()
                       ) + make_interval(days => $4::int)
               RETURNING expires_at"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(item_key)
        .bind(days_to_add)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.0)
    }

    async fn purge_expired(&self) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM coude_steal_boosts WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(result.rows_affected())
    }
}
