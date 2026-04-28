use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use sqlx::Postgres;
use sqlx::Transaction;
use crate::domain::entities::casino::slot::SlotJackpotPool;
use crate::domain::entities::casino::slot::SlotSpin;
use crate::domain::entities::casino::slot::SlotTopWinner;
use crate::domain::errors::DomainError;
use crate::ports::outbound::casino::slot_repository::SlotRepository;

use super::super::pg_err;

pub struct PgSlotRepository {
    pool: PgPool,
}

impl PgSlotRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SlotRepository for PgSlotRepository {
    async fn get_jackpot_pool(&self, guild_id: &str) -> Result<Option<SlotJackpotPool>, DomainError> {
        let row: Option<(String, i64, Option<String>, Option<DateTime<Utc>>, Option<i64>)> =
            sqlx::query_as(
                "SELECT guild_id, current_pool, last_won_by, last_won_at, last_won_amount
                 FROM slot_jackpot_pool WHERE guild_id = $1",
            )
            .bind(guild_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(row.map(|(gid, cp, lwb, lwa, lwam)| SlotJackpotPool {
            guild_id: gid,
            current_pool: cp,
            last_won_by: lwb,
            last_won_at: lwa,
            last_won_amount: lwam,
        }))
    }

    async fn init_jackpot_pool_if_absent(&self, guild_id: &str, starting: i64) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO slot_jackpot_pool (guild_id, current_pool)
             VALUES ($1, $2)
             ON CONFLICT (guild_id) DO NOTHING",
        )
        .bind(guild_id)
        .bind(starting)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn add_to_jackpot_pool_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        guild_id: &str,
        amount: i64,
        starting: i64,
    ) -> Result<i64, DomainError> {
        let new_total: i64 = sqlx::query_scalar(
            "INSERT INTO slot_jackpot_pool (guild_id, current_pool)
             VALUES ($1, $2 + $3)
             ON CONFLICT (guild_id) DO UPDATE
                SET current_pool = slot_jackpot_pool.current_pool + EXCLUDED.current_pool - $2,
                    updated_at = NOW()
             RETURNING current_pool",
        )
        .bind(guild_id)
        .bind(starting)
        .bind(amount)
        .fetch_one(&mut **tx)
        .await
        .map_err(pg_err)?;
        Ok(new_total)
    }

    async fn claim_jackpot_pool_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        guild_id: &str,
        winner_id: &str,
        won_amount: i64,
        reset_to: i64,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE slot_jackpot_pool
             SET current_pool = $2,
                 last_won_by = $3,
                 last_won_at = NOW(),
                 last_won_amount = $4,
                 updated_at = NOW()
             WHERE guild_id = $1",
        )
        .bind(guild_id)
        .bind(reset_to)
        .bind(winner_id)
        .bind(won_amount)
        .execute(&mut **tx)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn log_spin_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        spin: &SlotSpin,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO slot_spin_log
             (id, guild_id, user_id, username, mise, symbols, payout, multiplier, is_jackpot, is_free, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(spin.id)
        .bind(&spin.guild_id)
        .bind(&spin.user_id)
        .bind(&spin.username)
        .bind(spin.mise)
        .bind(serde_json::to_value(&spin.symbols).unwrap_or(serde_json::Value::Null))
        .bind(spin.payout)
        .bind(spin.multiplier as f32)
        .bind(spin.is_jackpot)
        .bind(spin.is_free)
        .bind(spin.created_at)
        .execute(&mut **tx)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn last_spin_at(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        let ts: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT MAX(created_at) FROM slot_spin_log
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?
        .flatten();
        Ok(ts)
    }

    async fn has_claimed_daily_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<bool, DomainError> {
        let exists: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM slot_daily_claims
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

    async fn mark_daily_claimed_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO slot_daily_claims (guild_id, user_id, day)
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
    ) -> Result<Vec<SlotSpin>, DomainError> {
        let rows: Vec<(uuid::Uuid, String, String, String, i64, serde_json::Value, i64, f32, bool, bool, DateTime<Utc>)> =
            sqlx::query_as(
                "SELECT id, guild_id, user_id, username, mise, symbols, payout, multiplier,
                        is_jackpot, is_free, created_at
                 FROM slot_spin_log
                 WHERE guild_id = $1
                 ORDER BY created_at DESC
                 LIMIT $2",
            )
            .bind(guild_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(rows.into_iter().map(|r| SlotSpin {
            id: r.0,
            guild_id: r.1,
            user_id: r.2,
            username: r.3,
            mise: r.4,
            symbols: serde_json::from_value(r.5).unwrap_or_default(),
            payout: r.6,
            multiplier: r.7 as f64,
            is_jackpot: r.8,
            is_free: r.9,
            created_at: r.10,
        }).collect())
    }

    async fn top_winners(
        &self,
        guild_id: &str,
        days: i64,
        limit: i64,
    ) -> Result<Vec<SlotTopWinner>, DomainError> {
        let rows: Vec<(String, String, i64, i64, i64)> = sqlx::query_as(
            "SELECT user_id,
                    COALESCE(MAX(username), user_id) AS username,
                    COALESCE(SUM(payout), 0)::bigint AS total_payout,
                    COUNT(*) FILTER (WHERE is_jackpot)::bigint AS jackpot_count,
                    COUNT(*)::bigint AS spin_count
             FROM slot_spin_log
             WHERE guild_id = $1
               AND created_at >= NOW() - ($2 || ' days')::interval
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

        Ok(rows.into_iter().map(|(uid, name, total, jackpots, spins)| SlotTopWinner {
            user_id: uid,
            username: name,
            total_payout: total,
            jackpot_count: jackpots as u32,
            spin_count: spins as u32,
        }).collect())
    }
}
