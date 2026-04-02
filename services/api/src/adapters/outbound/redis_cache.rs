use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use redis::AsyncCommands;

use crate::domain::entities::Rule;
use crate::domain::errors::DomainError;
use crate::domain::value_objects::FlagType;
use crate::ports::outbound::CachePort;

const RULES_TTL: u64 = 300; // 5 minutes

pub struct RedisCache {
    #[allow(dead_code)]
    client: redis::Client,
    /// Connexion multiplexee persistante (cloneable, partage le meme socket TCP).
    conn: redis::aio::MultiplexedConnection,
    /// Compteur de cache hits.
    hits: AtomicU64,
    /// Compteur de cache misses.
    misses: AtomicU64,
}

/// Statistiques du cache Redis.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total: u64,
    pub hit_rate_percent: f64,
}

impl RedisCache {
    pub async fn new(client: redis::Client) -> Result<Self, redis::RedisError> {
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self {
            client,
            conn,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        })
    }

    fn rules_key(guild_id: &str) -> String {
        format!("rules:{guild_id}")
    }

    async fn conn(&self) -> Result<redis::aio::MultiplexedConnection, DomainError> {
        Ok(self.conn.clone())
    }

    fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Retourne les statistiques du cache (hits, misses, hit rate).
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        CacheStats {
            hits,
            misses,
            total,
            hit_rate_percent: (hit_rate * 10.0).round() / 10.0,
        }
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
                self.record_hit();
                let cached: Vec<CachedRule> = serde_json::from_str(&json)
                    .map_err(|e| DomainError::Internal(format!("Redis deserialize: {e}")))?;
                Ok(Some(cached.into_iter().map(|c| c.to_rule()).collect()))
            }
            None => {
                self.record_miss();
                Ok(None)
            }
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

        let result: Option<String> = conn.get(key)
            .await
            .map_err(|e| DomainError::Internal(format!("Redis GET {key}: {e}")))?;

        match &result {
            Some(_) => self.record_hit(),
            None => self.record_miss(),
        }

        Ok(result)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_stats_initial() {
        let stats = CacheStats {
            hits: 0,
            misses: 0,
            total: 0,
            hit_rate_percent: 0.0,
        };
        assert_eq!(stats.hit_rate_percent, 0.0);
    }

    #[test]
    fn cache_stats_computation() {
        let stats = CacheStats {
            hits: 80,
            misses: 20,
            total: 100,
            hit_rate_percent: 80.0,
        };
        assert_eq!(stats.hits, 80);
        assert_eq!(stats.misses, 20);
        assert_eq!(stats.hit_rate_percent, 80.0);
    }

    #[test]
    fn cache_stats_serializes() {
        let stats = CacheStats {
            hits: 42,
            misses: 8,
            total: 50,
            hit_rate_percent: 84.0,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"hits\":42"));
        assert!(json.contains("\"hit_rate_percent\":84.0"));
    }
}
