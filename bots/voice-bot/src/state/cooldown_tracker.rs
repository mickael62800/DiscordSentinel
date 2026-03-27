use std::time::Instant;

use dashmap::DashMap;
use serenity::model::id::UserId;

const COOLDOWN_SECS: u64 = 5;

pub struct CooldownTracker {
    map: DashMap<UserId, Instant>,
}

impl CooldownTracker {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    /// Verifie le cooldown. Retourne Some(remaining_secs) si en cooldown, None si OK.
    pub fn check(&self, user_id: UserId) -> Option<u64> {
        if let Some(entry) = self.map.get(&user_id) {
            let elapsed = entry.value().elapsed().as_secs();
            if elapsed < COOLDOWN_SECS {
                return Some(COOLDOWN_SECS - elapsed);
            }
        }
        None
    }

    /// Enregistre le timestamp de creation.
    pub fn set(&self, user_id: UserId) {
        self.map.insert(user_id, Instant::now());
    }

    /// Nettoie les entrees expirees.
    #[allow(dead_code)]
    pub fn cleanup(&self) {
        self.map.retain(|_, instant| instant.elapsed().as_secs() < COOLDOWN_SECS * 2);
    }
}
