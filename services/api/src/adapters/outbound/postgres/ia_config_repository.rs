use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::entities::IaConfig;
use crate::domain::errors::DomainError;
use crate::ports::outbound::IaConfigRepository;

pub struct PgIaConfigRepository {
    pool: PgPool,
}

impl PgIaConfigRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IaConfigRepository for PgIaConfigRepository {
    async fn get(&self, guild_id: &str) -> Result<Option<IaConfig>, DomainError> {
        let row = sqlx::query_as::<_, IaConfig>(
            "SELECT * FROM ia_config WHERE guild_id = $1",
        )
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row)
    }

    async fn save(&self, config: &IaConfig) -> Result<IaConfig, DomainError> {
        let row = sqlx::query_as::<_, IaConfig>(
            "INSERT INTO ia_config (guild_id, text_enabled, text_threshold, vision_enabled, vision_threshold, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW()) \
             ON CONFLICT (guild_id) DO UPDATE SET \
               text_enabled = $2, text_threshold = $3, \
               vision_enabled = $4, vision_threshold = $5, \
               updated_at = NOW() \
             RETURNING *",
        )
        .bind(&config.guild_id)
        .bind(config.text_enabled)
        .bind(config.text_threshold)
        .bind(config.vision_enabled)
        .bind(config.vision_threshold)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row)
    }
}
