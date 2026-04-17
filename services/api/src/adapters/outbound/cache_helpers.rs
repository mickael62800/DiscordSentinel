//! Helpers generiques pour le pattern cache-aside JSON.
//!
//! Evite de repeter le boilerplate `serde_json::{from_str, to_string}` +
//! `get_json` / `set_json` dans chaque service applicatif.
//!
//! Usage typique :
//!
//! ```ignore
//! let tickets = cached_json(&self.cache, &cache_key, TTL, || async {
//!     self.ticket_repo.find_all(...).await
//! }).await?;
//! ```

use std::future::Future;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::domain::errors::DomainError;
use crate::ports::outbound::CachePort;

/// Pattern cache-aside : lit depuis le cache, sinon execute `fetch` et ecrit
/// le resultat dans le cache avec le TTL specifie.
///
/// Semantique :
/// - Un echec Redis `GET` propage l'erreur (comportement existant des services).
/// - Un JSON invalide en cache est silencieusement ignore, on fallback sur `fetch`.
/// - Un echec Redis `SETEX` est logue mais n'empeche pas le retour de la valeur.
/// - Un echec de serialisation JSON est silencieusement ignore (pas de set).
pub async fn cached_json<T, F, Fut>(
    cache: &Arc<dyn CachePort>,
    key: &str,
    ttl_secs: u64,
    fetch: F,
) -> Result<T, DomainError>
where
    T: Serialize + DeserializeOwned,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, DomainError>>,
{
    if let Some(json) = cache.get_json(key).await? {
        if let Ok(data) = serde_json::from_str::<T>(&json) {
            return Ok(data);
        }
    }

    let data = fetch().await?;

    if let Ok(json) = serde_json::to_string(&data) {
        if let Err(e) = cache.set_json(key, &json, ttl_secs).await {
            tracing::warn!(error = %e, cache_key = %key, "Echec cache set (cached_json)");
        }
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::domain::entities::Rule;

    #[derive(Default)]
    struct MemoryCache {
        data: std::sync::Mutex<std::collections::HashMap<String, String>>,
        get_calls: AtomicUsize,
        set_calls: AtomicUsize,
    }

    #[async_trait]
    impl CachePort for MemoryCache {
        async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> {
            Ok(None)
        }
        async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> {
            Ok(())
        }
        async fn invalidate_rules(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn get_json(&self, key: &str) -> Result<Option<String>, DomainError> {
            self.get_calls.fetch_add(1, Ordering::Relaxed);
            Ok(self.data.lock().unwrap().get(key).cloned())
        }
        async fn set_json(&self, key: &str, json: &str, _ttl: u64) -> Result<(), DomainError> {
            self.set_calls.fetch_add(1, Ordering::Relaxed);
            self.data.lock().unwrap().insert(key.to_string(), json.to_string());
            Ok(())
        }
        async fn invalidate(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn invalidate_pattern(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn cached_json_cache_miss_fetches_and_stores() {
        let cache: Arc<dyn CachePort> = Arc::new(MemoryCache::default());
        let result: Result<Vec<i32>, DomainError> =
            cached_json(&cache, "test:key", 60, || async { Ok(vec![1, 2, 3]) }).await;
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn cached_json_cache_hit_skips_fetch() {
        let mem = Arc::new(MemoryCache::default());
        // pre-populate
        mem.data
            .lock()
            .unwrap()
            .insert("test:key".to_string(), "[9,8,7]".to_string());
        let cache: Arc<dyn CachePort> = mem;

        let fetched = AtomicUsize::new(0);
        let result: Result<Vec<i32>, DomainError> =
            cached_json(&cache, "test:key", 60, || async {
                fetched.fetch_add(1, Ordering::Relaxed);
                Ok(vec![0])
            })
            .await;
        assert_eq!(result.unwrap(), vec![9, 8, 7]);
        assert_eq!(fetched.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn cached_json_invalid_json_falls_back_to_fetch() {
        let mem = Arc::new(MemoryCache::default());
        mem.data
            .lock()
            .unwrap()
            .insert("test:key".to_string(), "not-json".to_string());
        let cache: Arc<dyn CachePort> = mem;

        let result: Result<Vec<i32>, DomainError> =
            cached_json(&cache, "test:key", 60, || async { Ok(vec![42]) }).await;
        assert_eq!(result.unwrap(), vec![42]);
    }
}
