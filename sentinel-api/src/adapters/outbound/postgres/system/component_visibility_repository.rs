//! Adapter sortant Postgres des overrides de visibilite des composants UI par
//! role (`rbac_component_visibility`). Tout le SQL du domaine (dont la
//! transaction batch) vit ici.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::adapters::outbound::postgres::pg_err;
use crate::ports::outbound::system::component_visibility_repository::ComponentVisibilityRepository;
use sentinel_core::domain::entities::system::component_visibility::VisibilityEntry;
use sentinel_core::domain::errors::DomainError;

pub struct PgComponentVisibilityRepository {
    pool: PgPool,
}

impl PgComponentVisibilityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ComponentVisibilityRepository for PgComponentVisibilityRepository {
    async fn list(&self, guild_id: &str) -> Result<Vec<VisibilityEntry>, DomainError> {
        let rows = sqlx::query_as::<_, (String, String, bool)>(
            "SELECT component_key, role, visible \
             FROM rbac_component_visibility \
             WHERE guild_id = $1",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|(component_key, role, visible)| VisibilityEntry {
                component_key,
                role,
                visible,
            })
            .collect())
    }

    async fn upsert_batch(
        &self,
        guild_id: &str,
        entries: &[VisibilityEntry],
        updated_by: &str,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        for e in entries {
            sqlx::query(
                "INSERT INTO rbac_component_visibility \
                     (guild_id, component_key, role, visible, updated_at, updated_by) \
                 VALUES ($1, $2, $3, $4, NOW(), $5) \
                 ON CONFLICT (guild_id, component_key, role) DO UPDATE SET \
                     visible = EXCLUDED.visible, \
                     updated_at = NOW(), \
                     updated_by = EXCLUDED.updated_by",
            )
            .bind(guild_id)
            .bind(&e.component_key)
            .bind(&e.role)
            .bind(e.visible)
            .bind(updated_by)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;
        }

        tx.commit().await.map_err(pg_err)?;
        Ok(())
    }
}
