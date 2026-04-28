use async_trait::async_trait;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::moderation::strikes::StrikeConfig;
use crate::domain::entities::moderation::strikes::StrikeThreshold;
use crate::domain::entities::moderation::strikes::UserStrike;
use crate::domain::errors::DomainError;
use crate::ports::outbound::moderation::strike_repository::StrikeRepository;

pub struct PgStrikeRepository {
    pool: PgPool,
}

impl PgStrikeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct StrikeRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    reason: String,
    source: String,
    infraction_id: Option<Uuid>,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<StrikeRow> for UserStrike {
    fn from(r: StrikeRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            user_id: r.user_id,
            reason: r.reason,
            source: r.source,
            infraction_id: r.infraction_id,
            expires_at: r.expires_at,
            created_at: r.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StrikeConfigRow {
    guild_id: String,
    window_secs: i64,
    thresholds: serde_json::Value,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<StrikeConfigRow> for StrikeConfig {
    fn from(r: StrikeConfigRow) -> Self {
        let thresholds: Vec<StrikeThreshold> = match serde_json::from_value(r.thresholds.clone()) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(guild_id = %r.guild_id, error = %e, raw = %r.thresholds, "Parse thresholds JSON echoue, fallback vec![]");
                Vec::new()
            }
        };
        Self {
            guild_id: r.guild_id,
            window_secs: r.window_secs,
            thresholds,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl StrikeRepository for PgStrikeRepository {
    async fn save_strike(&self, strike: &UserStrike) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO user_strikes (id, guild_id, user_id, reason, source, infraction_id, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(strike.id)
        .bind(&strike.guild_id)
        .bind(&strike.user_id)
        .bind(&strike.reason)
        .bind(&strike.source)
        .bind(strike.infraction_id)
        .bind(strike.expires_at)
        .bind(strike.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("save_strike: {e}")))?;
        Ok(())
    }

    async fn find_active_strikes(&self, guild_id: &str, user_id: &str, window_secs: i64) -> Result<Vec<UserStrike>, DomainError> {
        let cutoff = Utc::now() - Duration::seconds(window_secs);
        let rows = sqlx::query_as::<_, StrikeRow>(
            "SELECT id, guild_id, user_id, reason, source, infraction_id, expires_at, created_at
             FROM user_strikes
             WHERE guild_id = $1 AND user_id = $2
               AND (expires_at IS NULL OR expires_at > NOW())
               AND created_at > $3
             ORDER BY created_at DESC"
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("find_active_strikes: {e}")))?;

        Ok(rows.into_iter().map(UserStrike::from).collect())
    }

    async fn delete_strikes(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM user_strikes WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(format!("delete_strikes: {e}")))?;
        Ok(())
    }

    async fn delete_strike_by_infraction_id(&self, infraction_id: Uuid) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM user_strikes WHERE infraction_id = $1")
            .bind(infraction_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(format!("delete_strike_by_infraction_id: {e}")))?;
        Ok(result.rows_affected())
    }

    async fn get_config(&self, guild_id: &str) -> Result<Option<StrikeConfig>, DomainError> {
        let row = sqlx::query_as::<_, StrikeConfigRow>(
            "SELECT guild_id, window_secs, thresholds, enabled, created_at, updated_at
             FROM strike_config WHERE guild_id = $1"
        )
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("get_strike_config: {e}")))?;

        Ok(row.map(StrikeConfig::from))
    }

    async fn save_config(&self, config: &StrikeConfig) -> Result<(), DomainError> {
        let thresholds_json = serde_json::to_value(&config.thresholds)
            .map_err(|e| DomainError::Internal(format!("serialize thresholds: {e}")))?;

        sqlx::query(
            "INSERT INTO strike_config (guild_id, window_secs, thresholds, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, NOW(), NOW())
             ON CONFLICT (guild_id) DO UPDATE SET
               window_secs = EXCLUDED.window_secs,
               thresholds = EXCLUDED.thresholds,
               enabled = EXCLUDED.enabled,
               updated_at = NOW()"
        )
        .bind(&config.guild_id)
        .bind(config.window_secs)
        .bind(thresholds_json)
        .bind(config.enabled)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("save_strike_config: {e}")))?;

        Ok(())
    }
}
