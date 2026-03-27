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
