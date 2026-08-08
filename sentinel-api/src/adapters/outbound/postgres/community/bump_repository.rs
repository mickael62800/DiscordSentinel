//! Adapter sortant Postgres du module Bump (journal bump_events + etat/rappel
//! bump_guild_state). Tout le SQL du domaine bump vit ici.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::adapters::outbound::postgres::pg_err;
use sentinel_core::domain::entities::community::bump::{BumpState, DueReminder};
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::community::bump_repository::BumpRepository;

pub struct PgBumpRepository {
    pool: PgPool,
}

impl PgBumpRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BumpRepository for PgBumpRepository {
    async fn try_claim_slot(
        &self,
        guild_id: &str,
        provider: &str,
        channel_id: &str,
        cooldown_minutes: i64,
        reminder_enabled: bool,
    ) -> Result<bool, DomainError> {
        // CAS : l'upsert ne met a jour last_bump_at (et ne RETURNING) que si le
        // dernier bump du (guild, provider) date de plus de cooldown_minutes.
        let slot: Option<String> = sqlx::query_scalar(
            "INSERT INTO bump_guild_state (guild_id, provider, channel_id, last_bump_at, cooldown_minutes, reminder_enabled, reminder_sent, updated_at) \
             VALUES ($1,$2,$3,NOW(),$4,$5,FALSE,NOW()) \
             ON CONFLICT (guild_id, provider) DO UPDATE SET \
                channel_id = EXCLUDED.channel_id, last_bump_at = NOW(), \
                cooldown_minutes = EXCLUDED.cooldown_minutes, reminder_enabled = EXCLUDED.reminder_enabled, \
                reminder_sent = FALSE, updated_at = NOW() \
             WHERE bump_guild_state.last_bump_at < NOW() - make_interval(mins => $4::int) \
             RETURNING guild_id",
        )
        .bind(guild_id)
        .bind(provider)
        .bind(channel_id)
        .bind(cooldown_minutes as i32)
        .bind(reminder_enabled)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(slot.is_some())
    }

    async fn weekly_count(&self, guild_id: &str, user_id: &str) -> Result<i64, DomainError> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM bump_events \
             WHERE guild_id = $1 AND user_id = $2 AND bumped_at >= NOW() - INTERVAL '7 days'",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)
    }

    async fn total_count(&self, guild_id: &str, user_id: &str) -> Result<i64, DomainError> {
        sqlx::query_scalar("SELECT COUNT(*) FROM bump_events WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(pg_err)
    }

    async fn record_event(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        reward_coins: i64,
        weekly_index: i64,
        provider: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO bump_events (guild_id, user_id, username, reward_coins, weekly_index, provider) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(username)
        .bind(reward_coins as i32)
        .bind(weekly_index as i32)
        .bind(provider)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn due_reminders(&self) -> Result<Vec<DueReminder>, DomainError> {
        let rows: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT guild_id, channel_id, provider FROM bump_guild_state \
             WHERE reminder_enabled AND NOT reminder_sent AND channel_id <> '' \
               AND NOW() >= last_bump_at + make_interval(mins => cooldown_minutes)",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|(guild_id, channel_id, provider)| DueReminder {
                guild_id,
                channel_id,
                provider,
            })
            .collect())
    }

    async fn mark_reminder_sent(
        &self,
        guild_id: &str,
        provider: Option<&str>,
    ) -> Result<(), DomainError> {
        match provider {
            Some(p) => {
                sqlx::query(
                    "UPDATE bump_guild_state SET reminder_sent = TRUE, updated_at = NOW() \
                     WHERE guild_id = $1 AND provider = $2",
                )
                .bind(guild_id)
                .bind(p)
                .execute(&self.pool)
                .await
                .map_err(pg_err)?;
            }
            None => {
                sqlx::query(
                    "UPDATE bump_guild_state SET reminder_sent = TRUE, updated_at = NOW() \
                     WHERE guild_id = $1",
                )
                .bind(guild_id)
                .execute(&self.pool)
                .await
                .map_err(pg_err)?;
            }
        }
        Ok(())
    }

    async fn guild_states(&self, guild_id: &str) -> Result<Vec<BumpState>, DomainError> {
        let rows: Vec<(String, String, chrono::DateTime<chrono::Utc>, i32)> = sqlx::query_as(
            "SELECT provider, channel_id, last_bump_at, cooldown_minutes \
             FROM bump_guild_state WHERE guild_id = $1 ORDER BY provider",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(
                |(provider, channel_id, last_bump_at, cooldown_minutes)| BumpState {
                    provider,
                    channel_id,
                    last_bump_at,
                    cooldown_minutes: cooldown_minutes as i64,
                },
            )
            .collect())
    }
}
