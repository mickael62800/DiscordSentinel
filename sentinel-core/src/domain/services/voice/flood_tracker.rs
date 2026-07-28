use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use dashmap::DashMap;

const DEFAULT_MAX_MESSAGES: u64 = 5;
const DEFAULT_TIME_WINDOW_SECS: u64 = 5;

/// Détecteur de flood par fenêtre glissante, par couple (salon, utilisateur).
/// Générique sur les clés `C` (salon) et `U` (utilisateur) — le core ne
/// connaît pas Discord. Seuils reconfigurables à chaud depuis la config API.
pub struct FloodTracker<C: Eq + Hash, U: Eq + Hash> {
    map: DashMap<(C, U), Vec<Instant>>,
    max_messages: AtomicU64,
    time_window_secs: AtomicU64,
}

impl<C: Eq + Hash, U: Eq + Hash> FloodTracker<C, U> {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            max_messages: AtomicU64::new(DEFAULT_MAX_MESSAGES),
            time_window_secs: AtomicU64::new(DEFAULT_TIME_WINDOW_SECS),
        }
    }

    /// Met a jour les seuils depuis la config API.
    pub fn set_thresholds(&self, max_messages: u64, time_window_secs: u64) {
        self.max_messages.store(max_messages, Ordering::Relaxed);
        self.time_window_secs
            .store(time_window_secs, Ordering::Relaxed);
    }

    /// Enregistre un message. Retourne true si flood detecte.
    pub fn record_message(&self, channel_id: C, user_id: U) -> bool {
        let key = (channel_id, user_id);
        let now = Instant::now();
        let window = self.time_window_secs.load(Ordering::Relaxed);
        let max = self.max_messages.load(Ordering::Relaxed) as usize;

        let mut entry = self.map.entry(key).or_default();
        let timestamps = entry.value_mut();
        timestamps.retain(|t| now.duration_since(*t).as_secs() < window);
        timestamps.push(now);
        timestamps.len() >= max
    }

    /// Nettoie le compteur pour un utilisateur dans un channel.
    pub fn clear(&self, channel_id: C, user_id: U) {
        self.map.remove(&(channel_id, user_id));
    }
}

impl<C: Eq + Hash, U: Eq + Hash> Default for FloodTracker<C, U> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Tracker = FloodTracker<u64, u64>;

    #[test]
    fn test_no_flood_below_threshold() {
        let tracker = Tracker::new();
        for _ in 0..(DEFAULT_MAX_MESSAGES - 1) {
            assert!(!tracker.record_message(1, 1));
        }
    }

    #[test]
    fn test_flood_at_threshold() {
        let tracker = Tracker::new();
        for i in 0..DEFAULT_MAX_MESSAGES {
            let result = tracker.record_message(1, 1);
            if i < DEFAULT_MAX_MESSAGES - 1 {
                assert!(!result);
            } else {
                assert!(result);
            }
        }
    }

    #[test]
    fn test_different_users_independent() {
        let tracker = Tracker::new();
        for _ in 0..(DEFAULT_MAX_MESSAGES - 1) {
            tracker.record_message(1, 1);
        }
        assert!(!tracker.record_message(1, 2));
    }

    #[test]
    fn test_clear_resets() {
        let tracker = Tracker::new();
        for _ in 0..(DEFAULT_MAX_MESSAGES - 1) {
            tracker.record_message(1, 1);
        }
        tracker.clear(1, 1);
        assert!(!tracker.record_message(1, 1));
    }
}
