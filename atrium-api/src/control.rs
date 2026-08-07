//! Etat active/desactive d'Atrium, persistant par serveur.

use sqlx::PgPool;

use crate::AppConfig;

#[derive(Clone)]
pub struct BotControlStore {
    pool: PgPool,
}

impl BotControlStore {
    pub fn new(config: &AppConfig) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PgPool::connect_lazy(&config.rag_database_url)?,
        })
    }

    pub async fn is_enabled(&self, guild_id: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT enabled FROM atrium_guild_settings WHERE guild_id = $1",
        )
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await
        .map(|value| value.unwrap_or(true))
    }

    pub async fn set_enabled(
        &self,
        guild_id: &str,
        enabled: bool,
        actor_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO atrium_guild_settings (guild_id, enabled, updated_by) VALUES ($1, $2, $3) \
             ON CONFLICT (guild_id) DO UPDATE SET enabled = EXCLUDED.enabled, \
             updated_by = EXCLUDED.updated_by, updated_at = now()",
        )
        .bind(guild_id)
        .bind(enabled)
        .bind(actor_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
