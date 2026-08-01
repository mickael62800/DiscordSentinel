//! Adapter Postgres du port `WheelRepository`.

use async_trait::async_trait;
use nexus_core::domain::entities::wheel::WheelSpin;
use nexus_core::domain::errors::DomainError;
use nexus_core::ports::outbound::wheel_repository::WheelRepository;
use sqlx::PgPool;

use super::pg_err;

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
    async fn try_claim(
        &self,
        guild_id: &str,
        user_id: &str,
        cooldown_hours: i64,
    ) -> Result<bool, DomainError> {
        // L'insertion CONDITIONNELLE fait office de verrou : le `NOT EXISTS`
        // et l'insertion sont evalues dans la meme instruction, donc deux
        // clics simultanes ne peuvent pas accorder deux tirages.
        //
        // `NOW()` cote base et non une date calculee en Rust : deux processus
        // aux horloges legerement decalees appliqueraient sinon des delais
        // differents.
        let res = sqlx::query(
            "INSERT INTO nexus_wheel_daily_claims (guild_id, user_id, day)
             SELECT $1, $2, CURRENT_DATE
             WHERE NOT EXISTS (
                 SELECT 1 FROM nexus_wheel_daily_claims
                 WHERE guild_id = $1 AND user_id = $2
                   AND claimed_at > NOW() - make_interval(hours => $3::int)
             )",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(cooldown_hours.clamp(1, 8760) as i32)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn has_claimed_recently(
        &self,
        guild_id: &str,
        user_id: &str,
        cooldown_hours: i64,
    ) -> Result<bool, DomainError> {
        // Meme reference de temps que `try_claim` : les deux doivent
        // s'accorder, sinon le bouton s'ouvre alors que le tirage sera refuse.
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM nexus_wheel_daily_claims
             WHERE guild_id = $1 AND user_id = $2
               AND claimed_at > NOW() - make_interval(hours => $3::int)
             LIMIT 1",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(cooldown_hours.clamp(1, 8760) as i32)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.is_some())
    }

    async fn log_spin(&self, spin: &WheelSpin) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO nexus_wheel_spin_log
             (id, guild_id, user_id, username, case_key, case_label, payout, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(spin.id)
        .bind(&spin.guild_id)
        .bind(&spin.user_id)
        .bind(&spin.username)
        .bind(&spin.case_key)
        .bind(&spin.case_label)
        .bind(spin.payout)
        .bind(spin.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
