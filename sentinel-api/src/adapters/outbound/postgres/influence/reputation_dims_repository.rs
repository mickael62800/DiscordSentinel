//! Impl Postgres de `ReputationDimsRepository`.

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::influence::reputation_dims::{ReputationDim, ReputationDims};
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::influence::reputation_dims_repository::ReputationDimsRepository;

use super::super::pg_err_ctx;

const TBL: &str = "influence_reputation_dims";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgReputationDimsRepository {
    pool: PgPool,
}

impl PgReputationDimsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReputationDimsRepository for PgReputationDimsRepository {
    async fn get(&self, citizen_id: Uuid) -> Result<ReputationDims, DomainError> {
        let row: Option<(i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT reliability, popularity, notoriety, transparency \
             FROM influence_reputation_dims WHERE citizen_id = $1",
        )
        .bind(citizen_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row
            .map(|(r, p, n, t)| ReputationDims {
                reliability: r,
                popularity: p,
                notoriety: n,
                transparency: t,
            })
            .unwrap_or_default())
    }

    async fn adjust(
        &self,
        citizen_id: Uuid,
        dim: ReputationDim,
        delta: i64,
    ) -> Result<i64, DomainError> {
        // `col` vient d'un enum ferme (jamais d'entree externe) : pas d'injection.
        let col = dim.column();
        let sql = format!(
            "INSERT INTO influence_reputation_dims (citizen_id, {col}) VALUES ($1, $2) \
             ON CONFLICT (citizen_id) DO UPDATE \
             SET {col} = influence_reputation_dims.{col} + $2, updated_at = NOW() \
             RETURNING {col}"
        );
        let v: i64 = sqlx::query_scalar(&sql)
            .bind(citizen_id)
            .bind(delta)
            .fetch_one(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(v)
    }
}
