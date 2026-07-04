//! Impl Postgres de `MovementRepository`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::influence::capital::Capital;
use sentinel_core::domain::entities::influence::movement::CapitalMovement;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::influence::movement_repository::MovementRepository;

use super::super::pg_err_ctx;

const TBL: &str = "influence_capital_movements";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgMovementRepository {
    pool: PgPool,
}

impl PgMovementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    capital: String,
    delta: i64,
    reason: String,
    created_at: DateTime<Utc>,
}

#[async_trait]
impl MovementRepository for PgMovementRepository {
    async fn record(
        &self,
        guild_id: &str,
        citizen_id: Uuid,
        capital: Capital,
        delta: i64,
        reason: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO influence_capital_movements (guild_id, citizen_id, capital, delta, reason) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(guild_id)
        .bind(citizen_id)
        .bind(capital.as_str())
        .bind(delta)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_recent(
        &self,
        citizen_id: Uuid,
        limit: i64,
    ) -> Result<Vec<CapitalMovement>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, capital, delta, reason, created_at FROM influence_capital_movements \
             WHERE citizen_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(citizen_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        rows.into_iter()
            .map(|r| {
                let capital = Capital::from_str_lossy(&r.capital).ok_or_else(|| {
                    DomainError::Internal(format!("capital inconnu : {}", r.capital))
                })?;
                Ok(CapitalMovement {
                    id: r.id,
                    capital,
                    delta: r.delta,
                    reason: r.reason,
                    created_at: r.created_at,
                })
            })
            .collect()
    }
}
