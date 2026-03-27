use std::time::Instant;

use dashmap::DashMap;
use serenity::model::id::{ChannelId, UserId};

const MAX_MESSAGES: usize = 5;
const TIME_WINDOW_SECS: u64 = 5;

pub struct FloodTracker {
    map: DashMap<(ChannelId, UserId), Vec<Instant>>,
}

impl FloodTracker {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    /// Enregistre un message. Retourne true si flood detecte (>= MAX_MESSAGES en TIME_WINDOW_SECS).
    pub fn record_message(&self, channel_id: ChannelId, user_id: UserId) -> bool {
        let key = (channel_id, user_id);
        let now = Instant::now();

        let mut entry = self.map.entry(key).or_default();
        let timestamps = entry.value_mut();
        timestamps.retain(|t| now.duration_since(*t).as_secs() < TIME_WINDOW_SECS);
        timestamps.push(now);
        timestamps.len() >= MAX_MESSAGES
    }

    /// Nettoie le compteur pour un utilisateur dans un channel.
    pub fn clear(&self, channel_id: ChannelId, user_id: UserId) {
        self.map.remove(&(channel_id, user_id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(id: u64) -> ChannelId { ChannelId::new(id) }
    fn uid(id: u64) -> UserId { UserId::new(id) }

    #[test]
    fn test_no_flood_below_threshold() {
        let tracker = FloodTracker::new();
        for _ in 0..(MAX_MESSAGES - 1) {
            assert!(!tracker.record_message(cid(1), uid(1)));
        }
    }

    #[test]
    fn test_flood_at_threshold() {
        let tracker = FloodTracker::new();
        for i in 0..MAX_MESSAGES {
            let result = tracker.record_message(cid(1), uid(1));
            if i < MAX_MESSAGES - 1 {
                assert!(!result);
            } else {
                assert!(result);
            }
        }
    }

    #[test]
    fn test_different_users_independent() {
        let tracker = FloodTracker::new();
        for _ in 0..(MAX_MESSAGES - 1) {
            tracker.record_message(cid(1), uid(1));
        }
        // User 2 dans le meme channel ne devrait pas etre en flood
        assert!(!tracker.record_message(cid(1), uid(2)));
    }

    #[test]
    fn test_clear_resets() {
        let tracker = FloodTracker::new();
        for _ in 0..(MAX_MESSAGES - 1) {
            tracker.record_message(cid(1), uid(1));
        }
        tracker.clear(cid(1), uid(1));
        assert!(!tracker.record_message(cid(1), uid(1)));
    }
}
