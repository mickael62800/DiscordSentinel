//! Adapter sortant Postgres du module Invitations (table `invitation_codes` +
//! octroi de role `api_user_guilds`). Tout le SQL du domaine invitation vit ici.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::adapters::outbound::postgres::pg_err;
use crate::ports::outbound::system::invitation_repository::InvitationRepository;
use sentinel_core::domain::entities::system::invitation::Invitation;
use sentinel_core::domain::errors::DomainError;

pub struct PgInvitationRepository {
    pool: PgPool,
}

impl PgInvitationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type InvitationRow = (
    String,
    String,
    String,
    String,
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<String>,
    Option<String>,
);

fn row_to_invitation(row: InvitationRow) -> Invitation {
    let (
        code,
        guild_id,
        role,
        created_by,
        created_at,
        expires_at,
        used_at,
        used_by_discord_id,
        notes,
    ) = row;
    Invitation {
        code,
        guild_id,
        role,
        created_by,
        created_at,
        expires_at,
        used_at,
        used_by_discord_id,
        notes,
    }
}

#[async_trait]
impl InvitationRepository for PgInvitationRepository {
    async fn code_exists(&self, code: &str) -> Result<bool, DomainError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT code FROM invitation_codes WHERE code = $1")
                .bind(code)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(row.is_some())
    }

    async fn insert_invitation(
        &self,
        code: &str,
        guild_id: &str,
        role: &str,
        created_by: &str,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        notes: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO invitation_codes \
             (code, guild_id, role, created_by, expires_at, notes) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(code)
        .bind(guild_id)
        .bind(role)
        .bind(created_by)
        .bind(expires_at)
        .bind(notes)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Invitation>, DomainError> {
        let rows = sqlx::query_as::<_, InvitationRow>(
            "SELECT code, guild_id, role, created_by, created_at, expires_at, used_at, used_by_discord_id, notes \
             FROM invitation_codes \
             WHERE guild_id = $1 \
             ORDER BY created_at DESC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(row_to_invitation).collect())
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<Invitation>, DomainError> {
        let row = sqlx::query_as::<_, InvitationRow>(
            "SELECT code, guild_id, role, created_by, created_at, expires_at, used_at, used_by_discord_id, notes \
             FROM invitation_codes \
             WHERE code = $1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(row_to_invitation))
    }

    async fn delete_unused(&self, code: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM invitation_codes WHERE code = $1 AND used_at IS NULL")
            .bind(code)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn count_user_guilds(&self, discord_user_id: &str) -> Result<i64, DomainError> {
        sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM api_user_guilds WHERE discord_user_id = $1",
        )
        .bind(discord_user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)
    }

    async fn redeem(
        &self,
        code: &str,
        discord_user_id: &str,
        guild_id: &str,
        role: &str,
    ) -> Result<bool, DomainError> {
        // Transaction : octroi RBAC + consommation du code, atomique.
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Insert ou update api_user_guilds.
        sqlx::query(
            "INSERT INTO api_user_guilds (discord_user_id, guild_id, role, granted_by, granted_at) \
             VALUES ($1, $2, $3, 'invitation', NOW()) \
             ON CONFLICT (discord_user_id, guild_id) DO UPDATE SET \
                 role = EXCLUDED.role, \
                 granted_by = 'invitation', \
                 granted_at = NOW()",
        )
        .bind(discord_user_id)
        .bind(guild_id)
        .bind(role)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        // Marquer le code consomme (atomic check-and-set).
        let updated = sqlx::query(
            "UPDATE invitation_codes SET used_at = NOW(), used_by_discord_id = $2 \
             WHERE code = $1 AND used_at IS NULL",
        )
        .bind(code)
        .bind(discord_user_id)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        if updated.rows_affected() == 0 {
            // Course : un autre user a consomme le code entre-temps. Rollback.
            tx.rollback().await.map_err(pg_err)?;
            return Ok(false);
        }

        tx.commit().await.map_err(pg_err)?;
        Ok(true)
    }
}
