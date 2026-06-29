use crate::ports::outbound::casino::wheel_repository::WheelRepository;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sentinel_core::domain::entities::casino::wheel::WheelSpin;
use sentinel_core::domain::entities::casino::wheel::WheelTopWinner;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::uow::DbTx;
use sqlx::PgPool;

use super::super::pg_err;
use super::super::uow::as_pg;

pub struct PgWheelRepository {
    pool: PgPool,
}

impl PgWheelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WheelRepository for PgWheelRepository {
    async fn has_claimed_today(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError> {
        let exists: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM wheel_daily_claims
             WHERE guild_id = $1 AND user_id = $2 AND day = CURRENT_DATE
             LIMIT 1",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(exists.is_some())
    }

    async fn log_spin_in_tx(&self, tx: &mut dyn DbTx, spin: &WheelSpin) -> Result<(), DomainError> {
        let tx = as_pg(tx);
        sqlx::query(
            "INSERT INTO wheel_spin_log
             (id, guild_id, user_id, username, case_key, case_label, payout, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(spin.id)
        .bind(spin.guild_id.as_str())
        .bind(spin.user_id.as_str())
        .bind(&spin.username)
        .bind(&spin.case_key)
        .bind(&spin.case_label)
        .bind(spin.payout)
        .bind(spin.created_at)
        .execute(&mut **tx)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn mark_claimed_in_tx(
        &self,
        tx: &mut dyn DbTx,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        let tx = as_pg(tx);
        sqlx::query(
            "INSERT INTO wheel_daily_claims (guild_id, user_id, day)
             VALUES ($1, $2, CURRENT_DATE)
             ON CONFLICT (guild_id, user_id, day) DO NOTHING",
        )
        .bind(guild_id)
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn recent_spins(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<WheelSpin>, DomainError> {
        let rows: Vec<(
            uuid::Uuid,
            String,
            String,
            String,
            String,
            String,
            i64,
            DateTime<Utc>,
        )> = sqlx::query_as(
            "SELECT id, guild_id, user_id, username, case_key, case_label, payout, created_at
                 FROM wheel_spin_log
                 WHERE guild_id = $1
                 ORDER BY created_at DESC
                 LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| WheelSpin {
                id: r.0,
                guild_id: r.1.into(),
                user_id: r.2.into(),
                username: r.3,
                case_key: r.4,
                case_label: r.5,
                payout: r.6,
                created_at: r.7,
            })
            .collect())
    }

    async fn top_winners(
        &self,
        guild_id: &str,
        days: i64,
        limit: i64,
    ) -> Result<Vec<WheelTopWinner>, DomainError> {
        let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT user_id,
                    COALESCE(MAX(username), user_id) AS username,
                    COALESCE(SUM(payout), 0)::bigint AS total_payout,
                    COUNT(*)::bigint AS spin_count
             FROM wheel_spin_log w
             WHERE guild_id = $1
               AND created_at >= NOW() - ($2 || ' days')::interval
               AND NOT EXISTS (SELECT 1 FROM guild_members gm
                               WHERE gm.guild_id = w.guild_id AND gm.user_id = w.user_id
                               AND gm.left_at IS NOT NULL)
             GROUP BY user_id
             ORDER BY total_payout DESC
             LIMIT $3",
        )
        .bind(guild_id)
        .bind(days.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|(uid, name, total, count)| WheelTopWinner {
                user_id: uid.into(),
                username: name,
                total_payout: total,
                spin_count: count as u32,
            })
            .collect())
    }
}
