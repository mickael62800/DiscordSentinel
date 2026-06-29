//! Impl Postgres de `RefusalCountRepository`.

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;

use crate::ports::outbound::coude::refusal_count_repository::RefusalCountRepository;
use sentinel_core::domain::entities::coude::refusal_count::RefusalCount;
use sentinel_core::domain::errors::DomainError;

use super::super::pg_err_ctx;
const TBL: &str = "coude_refusal_counts";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgRefusalCountRepository {
    pool: PgPool,
}

impl PgRefusalCountRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RefusalCountRepository for PgRefusalCountRepository {
    async fn increment(
        &self,
        guild_id: &str,
        requester_id: &str,
        refuser_id: &str,
    ) -> Result<i32, DomainError> {
        let row: (i32,) = sqlx::query_as(
            r#"INSERT INTO coude_refusal_counts (guild_id, requester_id, refuser_id, count)
               VALUES ($1, $2, $3, 1)
               ON CONFLICT (guild_id, requester_id, refuser_id) DO UPDATE
                   SET count = coude_refusal_counts.count + 1,
                       last_refused_at = NOW()
               RETURNING count"#,
        )
        .bind(guild_id)
        .bind(requester_id)
        .bind(refuser_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.0)
    }

    async fn get(
        &self,
        guild_id: &str,
        requester_id: &str,
        refuser_id: &str,
    ) -> Result<Option<RefusalCount>, DomainError> {
        let row: Option<(String, String, String, i32, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT guild_id, requester_id, refuser_id, count, last_refused_at
               FROM coude_refusal_counts
               WHERE guild_id = $1 AND requester_id = $2 AND refuser_id = $3"#,
        )
        .bind(guild_id)
        .bind(requester_id)
        .bind(refuser_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(|(g, r, f, c, t)| RefusalCount {
            guild_id: g.into(),
            requester_id: r,
            refuser_id: f,
            count: c,
            last_refused_at: t,
        }))
    }

    async fn reset(
        &self,
        guild_id: &str,
        requester_id: &str,
        refuser_id: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"UPDATE coude_refusal_counts
               SET count = 0
               WHERE guild_id = $1 AND requester_id = $2 AND refuser_id = $3"#,
        )
        .bind(guild_id)
        .bind(requester_id)
        .bind(refuser_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
