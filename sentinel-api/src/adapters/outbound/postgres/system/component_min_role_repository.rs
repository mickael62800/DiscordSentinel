//! Adapter sortant Postgres des overrides de min_role par composant
//! (`rbac_component_min_role`). Tout le SQL du domaine vit ici.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::adapters::outbound::postgres::pg_err;
use crate::ports::outbound::system::component_min_role_repository::ComponentMinRoleRepository;
use sentinel_core::domain::entities::system::component_min_role::ComponentMinRoleOverride;
use sentinel_core::domain::errors::DomainError;

pub struct PgComponentMinRoleRepository {
    pool: PgPool,
}

impl PgComponentMinRoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ComponentMinRoleRepository for PgComponentMinRoleRepository {
    async fn list_for_guild(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ComponentMinRoleOverride>, DomainError> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT component_key, min_role FROM rbac_component_min_role \
             WHERE guild_id = $1",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|(component_key, min_role)| ComponentMinRoleOverride {
                component_key,
                min_role,
            })
            .collect())
    }

    async fn get_override(
        &self,
        guild_id: &str,
        component_key: &str,
    ) -> Result<Option<String>, DomainError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT min_role FROM rbac_component_min_role \
             WHERE guild_id = $1 AND component_key = $2",
        )
        .bind(guild_id)
        .bind(component_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(|(min_role,)| min_role))
    }

    async fn upsert(
        &self,
        guild_id: &str,
        component_key: &str,
        min_role: &str,
        updated_by: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO rbac_component_min_role \
                 (guild_id, component_key, min_role, updated_at, updated_by) \
             VALUES ($1, $2, $3, NOW(), $4) \
             ON CONFLICT (guild_id, component_key) DO UPDATE SET \
                 min_role = EXCLUDED.min_role, \
                 updated_at = NOW(), \
                 updated_by = EXCLUDED.updated_by",
        )
        .bind(guild_id)
        .bind(component_key)
        .bind(min_role)
        .bind(updated_by)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn delete(&self, guild_id: &str, component_key: &str) -> Result<(), DomainError> {
        sqlx::query(
            "DELETE FROM rbac_component_min_role \
             WHERE guild_id = $1 AND component_key = $2",
        )
        .bind(guild_id)
        .bind(component_key)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
