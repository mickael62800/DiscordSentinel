use std::time::Instant;

use dashmap::DashMap;
use serenity::model::id::UserId;

pub struct AfkTracker {
    map: DashMap<UserId, Instant>,
}

impl AfkTracker {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    /// Marque un utilisateur comme AFK (mute + sourd).
    /// Ne met a jour que si pas deja traque.
    pub fn mark_afk(&self, user_id: UserId) {
        self.map.entry(user_id).or_insert_with(Instant::now);
    }

    /// Retire le marquage AFK (unmute, undeaf, ou leave).
    pub fn clear(&self, user_id: UserId) {
        self.map.remove(&user_id);
    }

    /// Retourne l'instant ou l'utilisateur est devenu AFK.
    #[allow(dead_code)]
    pub fn get_afk_since(&self, user_id: UserId) -> Option<Instant> {
        self.map.get(&user_id).map(|entry| *entry.value())
    }

    /// Retourne tous les utilisateurs AFK avec leur instant de debut.
    pub fn afk_users(&self) -> Vec<(UserId, Instant)> {
        self.map
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uid(id: u64) -> UserId {
        UserId::new(id)
    }

    #[test]
    fn test_not_afk_initially() {
        let tracker = AfkTracker::new();
        assert!(tracker.get_afk_since(uid(1)).is_none());
    }

    #[test]
    fn test_mark_afk() {
        let tracker = AfkTracker::new();
        tracker.mark_afk(uid(1));
        assert!(tracker.get_afk_since(uid(1)).is_some());
    }

    #[test]
    fn test_clear_afk() {
        let tracker = AfkTracker::new();
        tracker.mark_afk(uid(1));
        tracker.clear(uid(1));
        assert!(tracker.get_afk_since(uid(1)).is_none());
    }

    #[test]
    fn test_mark_does_not_reset_existing() {
        let tracker = AfkTracker::new();
        tracker.mark_afk(uid(1));
        let first = tracker.get_afk_since(uid(1)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        tracker.mark_afk(uid(1));
        let second = tracker.get_afk_since(uid(1)).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn test_different_users_independent() {
        let tracker = AfkTracker::new();
        tracker.mark_afk(uid(1));
        assert!(tracker.get_afk_since(uid(1)).is_some());
        assert!(tracker.get_afk_since(uid(2)).is_none());
    }

    #[test]
    fn test_afk_users_returns_all() {
        let tracker = AfkTracker::new();
        tracker.mark_afk(uid(1));
        tracker.mark_afk(uid(2));
        tracker.mark_afk(uid(3));
        let users = tracker.afk_users();
        assert_eq!(users.len(), 3);
    }

    #[test]
    fn test_afk_users_empty() {
        let tracker = AfkTracker::new();
        assert!(tracker.afk_users().is_empty());
    }
}
