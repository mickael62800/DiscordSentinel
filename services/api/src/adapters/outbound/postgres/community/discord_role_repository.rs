use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::entities::system::discord_role::DiscordRole;
use crate::domain::errors::DomainError;
use crate::ports::outbound::community::discord_role_repository::DiscordRoleRepository;

pub struct PgDiscordRoleRepository {
    pool: PgPool,
}

impl PgDiscordRoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DiscordRoleRepository for PgDiscordRoleRepository {
    async fn sync_roles(&self, guild_id: &str, roles: Vec<DiscordRole>) -> Result<(), DomainError> {
        // Supprimer les anciens roles du guild puis inserer les nouveaux
        let mut tx = self.pool.begin().await
            .map_err(|e| DomainError::Internal(format!("Transaction error: {e}")))?;

        sqlx::query("DELETE FROM discord_roles WHERE guild_id = $1")
            .bind(guild_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Internal(format!("Delete roles error: {e}")))?;

        for role in &roles {
            sqlx::query(
                "INSERT INTO discord_roles (id, guild_id, name, color, position, permissions, mentionable, managed, icon, member_count, synced_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())"
            )
            .bind(&role.id)
            .bind(guild_id)
            .bind(&role.name)
            .bind(role.color)
            .bind(role.position)
            .bind(role.permissions)
            .bind(role.mentionable)
            .bind(role.managed)
            .bind(&role.icon)
            .bind(role.member_count)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Internal(format!("Insert role error: {e}")))?;
        }

        tx.commit().await
            .map_err(|e| DomainError::Internal(format!("Commit error: {e}")))?;

        Ok(())
    }

    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<DiscordRole>, DomainError> {
        sqlx::query_as::<_, DiscordRole>(
            "SELECT id, guild_id, name, color, position, permissions, mentionable, managed, icon, member_count, synced_at \
             FROM discord_roles WHERE guild_id = $1 ORDER BY position DESC"
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("Query roles error: {e}")))
    }

    async fn find_by_id(&self, guild_id: &str, role_id: &str) -> Result<Option<DiscordRole>, DomainError> {
        sqlx::query_as::<_, DiscordRole>(
            "SELECT id, guild_id, name, color, position, permissions, mentionable, managed, icon, member_count, synced_at \
             FROM discord_roles WHERE guild_id = $1 AND id = $2"
        )
        .bind(guild_id)
        .bind(role_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("Query role error: {e}")))
    }
}
