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

    fn rules_key(guild_id: &str) -> String {
        format!("rules:{guild_id}")
    }

    async fn conn(&self) -> Result<redis::aio::MultiplexedConnection, DomainError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| DomainError::Internal(format!("Redis connection: {e}")))
    }
}

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
    // --- Rules cache ---

    async fn get_rules(&self, guild_id: &str) -> Result<Option<Vec<Rule>>, DomainError> {
        let mut conn = self.conn().await?;

        let data: Option<String> = conn
            .get(Self::rules_key(guild_id))
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
        let mut conn = self.conn().await?;

        let cached: Vec<CachedRule> = rules.iter().map(CachedRule::from).collect();
        let json =
            serde_json::to_string(&cached).map_err(|e| DomainError::Internal(e.to_string()))?;

        conn.set_ex::<_, _, ()>(Self::rules_key(guild_id), json, RULES_TTL)
            .await
            .map_err(|e| DomainError::Internal(format!("Redis SETEX: {e}")))?;

        Ok(())
    }

    async fn invalidate_rules(&self, guild_id: &str) -> Result<(), DomainError> {
        let mut conn = self.conn().await?;

        conn.del::<_, ()>(Self::rules_key(guild_id))
            .await
            .map_err(|e| DomainError::Internal(format!("Redis DEL: {e}")))?;

        Ok(())
    }

    // --- Generic JSON cache ---

    async fn get_json(&self, key: &str) -> Result<Option<String>, DomainError> {
        let mut conn = self.conn().await?;

        conn.get(key)
            .await
            .map_err(|e| DomainError::Internal(format!("Redis GET {key}: {e}")))
    }

    async fn set_json(&self, key: &str, json: &str, ttl_secs: u64) -> Result<(), DomainError> {
        let mut conn = self.conn().await?;

        conn.set_ex::<_, _, ()>(key, json, ttl_secs)
            .await
            .map_err(|e| DomainError::Internal(format!("Redis SETEX {key}: {e}")))?;

        Ok(())
    }

    async fn invalidate(&self, key: &str) -> Result<(), DomainError> {
        let mut conn = self.conn().await?;

        conn.del::<_, ()>(key)
            .await
            .map_err(|e| DomainError::Internal(format!("Redis DEL {key}: {e}")))?;

        Ok(())
    }

    async fn invalidate_pattern(&self, pattern: &str) -> Result<(), DomainError> {
        let mut conn = self.conn().await?;

        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| DomainError::Internal(format!("Redis KEYS {pattern}: {e}")))?;

        for key in keys {
            conn.del::<_, ()>(&key).await.ok();
        }

        Ok(())
    }
}
