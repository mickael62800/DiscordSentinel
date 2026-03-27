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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uid(id: u64) -> UserId { UserId::new(id) }

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
        assert!(remaining.unwrap() <= COOLDOWN_SECS);
    }

    #[test]
    fn test_different_users_independent() {
        let tracker = CooldownTracker::new();
        tracker.set(uid(1));
        assert!(tracker.check(uid(1)).is_some());
        assert!(tracker.check(uid(2)).is_none());
    }
}
