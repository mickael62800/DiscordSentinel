//! Adapter Postgres du port `RbacRepository` : RBAC applicatif
//! (`api_users`, `api_user_guilds`). Tout le SQL du domaine RBAC vit ici.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::super::pg_err_ctx;
use crate::ports::outbound::system::rbac_repository::RbacRepository;
use sentinel_core::domain::entities::system::rbac::GuildUserEntry;
use sentinel_core::domain::errors::DomainError;

const TBL: &str = "api_user_guilds";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgRbacRepository {
    pool: PgPool,
}

impl PgRbacRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct GuildUserRow {
    discord_user_id: String,
    display_name: String,
    avatar_url: Option<String>,
    role: String,
    granted_at: DateTime<Utc>,
    granted_by: Option<String>,
}

#[async_trait]
impl RbacRepository for PgRbacRepository {
    async fn upsert_user(&self, user_id: &str, display_name: &str) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO api_users (discord_user_id, display_name) \
             VALUES ($1, $2) \
             ON CONFLICT (discord_user_id) DO NOTHING",
        )
        .bind(user_id)
        .bind(display_name)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn insert_grant(
        &self,
        user_id: &str,
        guild_id: &str,
        role: &str,
        granted_by: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        let res: Result<(DateTime<Utc>,), sqlx::Error> = sqlx::query_as(
            "INSERT INTO api_user_guilds (discord_user_id, guild_id, role, granted_by) \
             VALUES ($1, $2, $3, $4) \
             RETURNING granted_at",
        )
        .bind(user_id)
        .bind(guild_id)
        .bind(role)
        .bind(granted_by)
        .fetch_one(&self.pool)
        .await;

        match res {
            Ok((granted_at,)) => Ok(Some(granted_at)),
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => Ok(None),
            Err(e) => Err(pg_err(e)),
        }
    }

    async fn update_role(
        &self,
        user_id: &str,
        guild_id: &str,
        role: &str,
    ) -> Result<u64, DomainError> {
        let res = sqlx::query(
            "UPDATE api_user_guilds SET role = $1 \
             WHERE discord_user_id = $2 AND guild_id = $3",
        )
        .bind(role)
        .bind(user_id)
        .bind(guild_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(res.rows_affected())
    }

    async fn count_owners(&self, guild_id: &str) -> Result<i64, DomainError> {
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM api_user_guilds \
             WHERE guild_id = $1 AND role = 'owner'",
        )
        .bind(guild_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(total)
    }

    async fn is_owner(&self, user_id: &str, guild_id: &str) -> Result<bool, DomainError> {
        let (exists,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM api_user_guilds \
             WHERE discord_user_id = $1 AND guild_id = $2 AND role = 'owner')",
        )
        .bind(user_id)
        .bind(guild_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(exists)
    }

    async fn delete_grant(&self, user_id: &str, guild_id: &str) -> Result<u64, DomainError> {
        let res =
            sqlx::query("DELETE FROM api_user_guilds WHERE discord_user_id = $1 AND guild_id = $2")
                .bind(user_id)
                .bind(guild_id)
                .execute(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(res.rows_affected())
    }

    async fn list_guild_users(
        &self,
        guild_id: &str,
    ) -> Result<Vec<GuildUserEntry>, DomainError> {
        let rows: Vec<GuildUserRow> = sqlx::query_as::<_, GuildUserRow>(
            "SELECT u.discord_user_id, u.display_name, u.avatar_url, \
                    g.role, g.granted_at, g.granted_by \
             FROM api_user_guilds g \
             INNER JOIN api_users u ON u.discord_user_id = g.discord_user_id \
             WHERE g.guild_id = $1 \
             ORDER BY \
                CASE g.role \
                    WHEN 'owner' THEN 0 \
                    WHEN 'admin' THEN 1 \
                    WHEN 'moderator' THEN 2 \
                    WHEN 'viewer' THEN 3 \
                END, \
                u.display_name ASC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| GuildUserEntry {
                discord_user_id: r.discord_user_id,
                display_name: r.display_name,
                avatar_url: r.avatar_url,
                role: r.role,
                granted_at: r.granted_at,
                granted_by: r.granted_by,
            })
            .collect())
    }
}
