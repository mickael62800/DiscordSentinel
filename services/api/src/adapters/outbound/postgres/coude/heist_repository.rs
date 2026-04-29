//! Impl Postgres de `HeistRepository` (Phase 10).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::coude::heist::HeistAttempt;
use crate::domain::entities::coude::heist::PrisonState;
use crate::domain::errors::DomainError;
use crate::ports::outbound::coude::heist_repository::HeistRepository;

use super::super::pg_err_ctx;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;
const TBL: &str = "heist";
fn pg_err(e: sqlx::Error) -> DomainError { pg_err_ctx(TBL, e) }

pub struct PgHeistRepository {
    pool: PgPool,
}

impl PgHeistRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct HeistRow {
    id: Uuid,
    guild_id: GuildId,
    user_id: UserId,
    success: bool,
    amount_stolen: i64,
    chance_percent: i32,
    tools_used: Vec<String>,
    attempted_at: DateTime<Utc>,
}

impl From<HeistRow> for HeistAttempt {
    fn from(r: HeistRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            user_id: r.user_id,
            success: r.success,
            amount_stolen: r.amount_stolen,
            chance_percent: r.chance_percent,
            tools_used: r.tools_used,
            attempted_at: r.attempted_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PrisonRow {
    guild_id: GuildId,
    user_id: UserId,
    released_at: DateTime<Utc>,
    reason: String,
    created_at: DateTime<Utc>,
}

impl From<PrisonRow> for PrisonState {
    fn from(r: PrisonRow) -> Self {
        Self {
            guild_id: r.guild_id,
            user_id: r.user_id,
            released_at: r.released_at,
            reason: r.reason,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl HeistRepository for PgHeistRepository {
    async fn last_attempt(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<HeistAttempt>, DomainError> {
        let row: Option<HeistRow> = sqlx::query_as(
            r#"SELECT id, guild_id, user_id, success, amount_stolen, chance_percent,
                      tools_used, attempted_at
               FROM coude_heist_attempts
               WHERE guild_id = $1 AND user_id = $2
               ORDER BY attempted_at DESC
               LIMIT 1"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn record_attempt(
        &self,
        guild_id: &str,
        user_id: &str,
        success: bool,
        amount_stolen: i64,
        chance_percent: i32,
        tools_used: &[String],
    ) -> Result<HeistAttempt, DomainError> {
        let row: HeistRow = sqlx::query_as(
            r#"INSERT INTO coude_heist_attempts
                 (guild_id, user_id, success, amount_stolen, chance_percent, tools_used)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id, guild_id, user_id, success, amount_stolen, chance_percent,
                         tools_used, attempted_at"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(success)
        .bind(amount_stolen)
        .bind(chance_percent)
        .bind(tools_used)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn get_prison(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<PrisonState>, DomainError> {
        let row: Option<PrisonRow> = sqlx::query_as(
            r#"SELECT guild_id, user_id, released_at, reason, created_at
               FROM coude_prison
               WHERE guild_id = $1 AND user_id = $2"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn send_to_prison(
        &self,
        guild_id: &str,
        user_id: &str,
        released_at: DateTime<Utc>,
        reason: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO coude_prison (guild_id, user_id, released_at, reason)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (guild_id, user_id) DO UPDATE
                 SET released_at = EXCLUDED.released_at,
                     reason = EXCLUDED.reason,
                     created_at = NOW()"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(released_at)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
