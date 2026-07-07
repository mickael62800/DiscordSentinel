//! Impl Postgres de `StealAttemptRepository` (`coude_steal_attempts`, Phase 5).

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::ports::outbound::coude::steal_attempt_repository::StealAttemptRepository;
use sentinel_core::domain::entities::coude::steal::attempt::NewStealAttempt;
use sentinel_core::domain::errors::DomainError;

use super::super::pg_ctx;

pub struct PgStealAttemptRepository {
    pool: PgPool,
}

impl PgStealAttemptRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StealAttemptRepository for PgStealAttemptRepository {
    async fn insert_pending(&self, attempt: &NewStealAttempt) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO coude_steal_attempts \
             (id, guild_id, thief_id, target_id, message_id, channel_id, expires_at, status) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending')",
        )
        .bind(attempt.id)
        .bind(&attempt.guild_id)
        .bind(&attempt.thief_id)
        .bind(&attempt.target_id)
        .bind(&attempt.message_id)
        .bind(&attempt.channel_id)
        .bind(attempt.expires_at)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("steal_attempts insert_pending"))?;
        Ok(())
    }

    async fn mark_defended(&self, id: Uuid) -> Result<bool, DomainError> {
        let res = sqlx::query(
            "UPDATE coude_steal_attempts SET status = 'defended' \
             WHERE id = $1 AND status = 'pending'",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("steal_attempts mark_defended"))?;
        Ok(res.rows_affected() == 1)
    }

    async fn claim_resolved(&self, id: Uuid) -> Result<bool, DomainError> {
        let res = sqlx::query(
            "UPDATE coude_steal_attempts SET status = 'resolved', resolved_at = NOW() \
             WHERE id = $1 AND status IN ('pending','defended','expired')",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("steal_attempts claim_resolved"))?;
        Ok(res.rows_affected() == 1)
    }
}
