use async_trait::async_trait;
use redis::AsyncCommands;

use crate::domain::entities::Rule;
use crate::domain::errors::DomainError;
use crate::domain::value_objects::FlagType;
use crate::ports::outbound::CachePort;

const RULES_TTL: u64 = 300; // 5 minutes

pub struct RedisCache {
    client: redis::Client,
}

impl RedisCache {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }

    fn key(guild_id: &str) -> String {
        format!("rules:{guild_id}")
    }
}

/// Représentation sérialisable d'une Rule pour le cache.
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedRule {
    id: String,
    guild_id: String,
    flag_type: String,
    weight: f64,
    threshold_warn: f64,
    threshold_delete: f64,
    threshold_mute: f64,
    threshold_ban: f64,
    enabled: bool,
}

impl From<&Rule> for CachedRule {
    fn from(r: &Rule) -> Self {
        Self {
            id: r.id.to_string(),
            guild_id: r.guild_id.clone(),
            flag_type: r.flag_type.as_str().to_string(),
            weight: r.weight,
            threshold_warn: r.threshold_warn,
            threshold_delete: r.threshold_delete,
            threshold_mute: r.threshold_mute,
            threshold_ban: r.threshold_ban,
            enabled: r.enabled,
        }
    }
}

impl CachedRule {
    fn to_rule(self) -> Rule {
        Rule {
            id: self.id.parse().unwrap_or_else(|_| {
                tracing::warn!("Invalid UUID in cache: {}, using nil", self.id);
                uuid::Uuid::nil()
            }),
            guild_id: self.guild_id,
            flag_type: FlagType::from_str_lossy(&self.flag_type),
            weight: self.weight,
            threshold_warn: self.threshold_warn,
            threshold_delete: self.threshold_delete,
            threshold_mute: self.threshold_mute,
            threshold_ban: self.threshold_ban,
            enabled: self.enabled,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}

#[async_trait]
impl CachePort for RedisCache {
    async fn get_rules(&self, guild_id: &str) -> Result<Option<Vec<Rule>>, DomainError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| DomainError::Internal(format!("Redis: {e}")))?;

        let data: Option<String> = conn
            .get(Self::key(guild_id))
            .await
            .map_err(|e| DomainError::Internal(format!("Redis GET: {e}")))?;

        match data {
            Some(json) => {
                let cached: Vec<CachedRule> = serde_json::from_str(&json)
                    .map_err(|e| DomainError::Internal(format!("Redis deserialize: {e}")))?;
                Ok(Some(cached.into_iter().map(|c| c.to_rule()).collect()))
            }
            None => Ok(None),
        }
    }

    async fn set_rules(&self, guild_id: &str, rules: &[Rule]) -> Result<(), DomainError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| DomainError::Internal(format!("Redis: {e}")))?;

        let cached: Vec<CachedRule> = rules.iter().map(CachedRule::from).collect();
        let json =
            serde_json::to_string(&cached).map_err(|e| DomainError::Internal(e.to_string()))?;

        conn.set_ex::<_, _, ()>(Self::key(guild_id), json, RULES_TTL)
            .await
            .map_err(|e| DomainError::Internal(format!("Redis SETEX: {e}")))?;

        Ok(())
    }

    async fn invalidate_rules(&self, guild_id: &str) -> Result<(), DomainError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| DomainError::Internal(format!("Redis: {e}")))?;

        conn.del::<_, ()>(Self::key(guild_id))
            .await
            .map_err(|e| DomainError::Internal(format!("Redis DEL: {e}")))?;

        Ok(())
    }
}
