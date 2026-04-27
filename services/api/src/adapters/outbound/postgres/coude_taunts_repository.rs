//! Impl Postgres de `CoudeTauntsRepository` (Phase 9 Part D).

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::entities::CoudeTauntsConfig;
use crate::domain::errors::DomainError;
use crate::ports::outbound::CoudeTauntsRepository;

use super::pg_err_ctx;
const TBL: &str = "taunts";
fn pg_err(e: sqlx::Error) -> DomainError { pg_err_ctx(TBL, e) }

pub struct PgCoudeTauntsRepository {
    pool: PgPool,
}

impl PgCoudeTauntsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CoudeTauntsRepository for PgCoudeTauntsRepository {
    async fn get_or_init_config(
        &self,
        guild_id: &str,
    ) -> Result<CoudeTauntsConfig, DomainError> {
        let row: (String, Option<String>, bool, bool, bool) = sqlx::query_as(
            r#"INSERT INTO coude_taunts_config (guild_id)
               VALUES ($1)
               ON CONFLICT (guild_id) DO UPDATE SET updated_at = coude_taunts_config.updated_at
               RETURNING guild_id, channel_id, enabled, rename_enabled, messages_enabled"#,
        )
        .bind(guild_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(CoudeTauntsConfig {
            guild_id: row.0,
            channel_id: row.1,
            enabled: row.2,
            rename_enabled: row.3,
            messages_enabled: row.4,
        })
    }

    async fn set_channel(
        &self,
        guild_id: &str,
        channel_id: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO coude_taunts_config (guild_id, channel_id)
               VALUES ($1, $2)
               ON CONFLICT (guild_id) DO UPDATE
                 SET channel_id = EXCLUDED.channel_id,
                     updated_at = NOW()"#,
        )
        .bind(guild_id)
        .bind(channel_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn set_enabled(&self, guild_id: &str, enabled: bool) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO coude_taunts_config (guild_id, enabled)
               VALUES ($1, $2)
               ON CONFLICT (guild_id) DO UPDATE
                 SET enabled = EXCLUDED.enabled,
                     updated_at = NOW()"#,
        )
        .bind(guild_id)
        .bind(enabled)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn set_rename_enabled(&self, guild_id: &str, rename_enabled: bool) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO coude_taunts_config (guild_id, rename_enabled)
               VALUES ($1, $2)
               ON CONFLICT (guild_id) DO UPDATE
                 SET rename_enabled = EXCLUDED.rename_enabled,
                     updated_at = NOW()"#,
        )
        .bind(guild_id)
        .bind(rename_enabled)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn set_messages_enabled(&self, guild_id: &str, messages_enabled: bool) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO coude_taunts_config (guild_id, messages_enabled)
               VALUES ($1, $2)
               ON CONFLICT (guild_id) DO UPDATE
                 SET messages_enabled = EXCLUDED.messages_enabled,
                     updated_at = NOW()"#,
        )
        .bind(guild_id)
        .bind(messages_enabled)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn is_opted_out(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError> {
        let row: Option<(bool,)> = sqlx::query_as(
            r#"SELECT TRUE FROM coude_taunts_opt_outs
               WHERE guild_id = $1 AND user_id = $2 LIMIT 1"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.is_some())
    }

    async fn list_opt_outs(&self, guild_id: &str) -> Result<Vec<String>, DomainError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"SELECT user_id FROM coude_taunts_opt_outs
               WHERE guild_id = $1
               ORDER BY created_at DESC"#,
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(|(u,)| u).collect())
    }

    async fn set_opt_out(
        &self,
        guild_id: &str,
        user_id: &str,
        opted_out: bool,
    ) -> Result<(), DomainError> {
        if opted_out {
            sqlx::query(
                r#"INSERT INTO coude_taunts_opt_outs (guild_id, user_id)
                   VALUES ($1, $2)
                   ON CONFLICT (guild_id, user_id) DO NOTHING"#,
            )
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        } else {
            sqlx::query(
                "DELETE FROM coude_taunts_opt_outs WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        }
        Ok(())
    }
}
