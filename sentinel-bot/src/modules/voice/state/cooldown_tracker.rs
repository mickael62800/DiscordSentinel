use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use dashmap::DashMap;
use serenity::model::id::UserId;

const DEFAULT_COOLDOWN_SECS: u64 = 5;

pub struct CooldownTracker {
    map: DashMap<UserId, Instant>,
    cooldown_secs: AtomicU64,
}

impl CooldownTracker {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            cooldown_secs: AtomicU64::new(DEFAULT_COOLDOWN_SECS),
        }
    }

    /// Met a jour le cooldown depuis la config API.
    pub fn set_cooldown_secs(&self, secs: u64) {
        self.cooldown_secs.store(secs, Ordering::Relaxed);
    }

    fn cooldown(&self) -> u64 {
        self.cooldown_secs.load(Ordering::Relaxed)
    }

    /// Verifie le cooldown. Retourne Some(remaining_secs) si en cooldown, None si OK.
    pub fn check(&self, user_id: UserId) -> Option<u64> {
        let cd = self.cooldown();
        if let Some(entry) = self.map.get(&user_id) {
            let elapsed = entry.value().elapsed().as_secs();
            if elapsed < cd {
                return Some(cd - elapsed);
            }
        }
        self.map.remove(&user_id);
        None
    }

    /// Enregistre le timestamp de creation.
    pub fn set(&self, user_id: UserId) {
        let cd = self.cooldown();
        if self.map.len() > 500 {
            self.map.retain(|_, ts| ts.elapsed().as_secs() < cd);
        }
        self.map.insert(user_id, Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uid(id: u64) -> UserId {
        UserId::new(id)
    }

    #[test]
    fn test_no_cooldown_initially() {
        let tracker = CooldownTracker::new();
        assert!(tracker.check(uid(1)).is_none());
    }

    #[test]
    fn test_cooldown_after_set() {
        let tracker = CooldownTracker::new();
        tracker.set(uid(1));
        let remaining = tracker.check(uid(1));
        assert!(remaining.is_some());
        assert!(remaining.unwrap() <= DEFAULT_COOLDOWN_SECS);
    }

    #[test]
    fn test_different_users_independent() {
        let tracker = CooldownTracker::new();
        tracker.set(uid(1));
        assert!(tracker.check(uid(1)).is_some());
        assert!(tracker.check(uid(2)).is_none());
    }
}
