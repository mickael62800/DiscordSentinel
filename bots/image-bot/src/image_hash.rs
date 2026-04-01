use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::time::{Duration, Instant};

use dashmap::DashMap;

/// Cache de hash d'images pour eviter d'analyser deux fois la meme image.
pub struct ImageHashCache {
    cache: DashMap<u64, (String, Instant)>,
    ttl: Duration,
}

impl ImageHashCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            cache: DashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Retourne le nombre d'entrees dans le cache.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Retourne l'action cached si pas expiree.
    pub fn get_cached(&self, hash: u64) -> Option<String> {
        let entry = self.cache.get(&hash)?;
        let (action, stored_at) = entry.value();
        if Instant::now().duration_since(*stored_at) < self.ttl {
            Some(action.clone())
        } else {
            drop(entry);
            self.cache.remove(&hash);
            None
        }
    }

    /// Stocke le resultat d'une analyse.
    pub fn store(&self, hash: u64, action: &str) {
        self.cache.insert(hash, (action.to_string(), Instant::now()));
    }
}

/// Hash rapide des bytes d'une image.
pub fn compute_hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_hash_deterministic() {
        let data = b"test image data";
        assert_eq!(compute_hash(data), compute_hash(data));
    }

    #[test]
    fn compute_hash_different_data() {
        assert_ne!(compute_hash(b"image A"), compute_hash(b"image B"));
    }

    #[test]
    fn cache_store_and_get() {
        let cache = ImageHashCache::new(300);
        cache.store(42, "delete");
        assert_eq!(cache.get_cached(42), Some("delete".to_string()));
    }

    #[test]
    fn cache_miss() {
        let cache = ImageHashCache::new(300);
        assert_eq!(cache.get_cached(99), None);
    }

    #[test]
    fn cache_expired() {
        let cache = ImageHashCache::new(0); // TTL = 0 → expire immediatement
        cache.store(42, "warn");
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert_eq!(cache.get_cached(42), None);
    }

    #[test]
    fn cache_overwrite() {
        let cache = ImageHashCache::new(300);
        cache.store(42, "warn");
        cache.store(42, "delete");
        assert_eq!(cache.get_cached(42), Some("delete".to_string()));
    }

    #[test]
    fn cache_multiple_entries() {
        let cache = ImageHashCache::new(300);
        cache.store(1, "none");
        cache.store(2, "warn");
        cache.store(3, "delete");
        assert_eq!(cache.get_cached(1), Some("none".to_string()));
        assert_eq!(cache.get_cached(2), Some("warn".to_string()));
        assert_eq!(cache.get_cached(3), Some("delete".to_string()));
    }
}
