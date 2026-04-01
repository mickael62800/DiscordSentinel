use std::time::{Duration, Instant};

use dashmap::DashMap;
use serenity::model::id::ChannelId;

/// Tracker d'activite par channel pour le slowmode adaptatif.
pub struct SlowmodeTracker {
    /// channel_id -> timestamps des messages recents
    counters: DashMap<ChannelId, Vec<Instant>>,
    window: Duration,
}

impl SlowmodeTracker {
    pub fn new(window_secs: u64) -> Self {
        Self {
            counters: DashMap::new(),
            window: Duration::from_secs(window_secs),
        }
    }

    /// Enregistre un message et retourne le nombre de messages dans la fenetre.
    pub fn record_message(&self, channel_id: ChannelId) -> usize {
        let now = Instant::now();
        let mut entry = self.counters.entry(channel_id).or_default();
        let timestamps = entry.value_mut();
        timestamps.retain(|t| now.duration_since(*t) < self.window);
        timestamps.push(now);
        timestamps.len()
    }

    /// Verifie si le seuil d'activation est atteint.
    pub fn should_activate(&self, channel_id: ChannelId, threshold: usize) -> bool {
        let now = Instant::now();
        self.counters
            .get(&channel_id)
            .map(|entry| {
                entry
                    .value()
                    .iter()
                    .filter(|t| now.duration_since(**t) < self.window)
                    .count()
                    >= threshold
            })
            .unwrap_or(false)
    }

    /// Reset le compteur d'un channel.
    #[allow(dead_code)]
    pub fn reset(&self, channel_id: ChannelId) {
        self.counters.remove(&channel_id);
    }

    /// Retourne le nombre de channels actuellement suivis.
    pub fn tracked_channels(&self) -> usize {
        self.counters.len()
    }

    /// Retourne le nombre de messages dans la fenetre pour un channel.
    #[allow(dead_code)]
    pub fn count(&self, channel_id: ChannelId) -> usize {
        let now = Instant::now();
        self.counters
            .get(&channel_id)
            .map(|entry| {
                entry
                    .value()
                    .iter()
                    .filter(|t| now.duration_since(**t) < self.window)
                    .count()
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_count() {
        let tracker = SlowmodeTracker::new(60);
        let ch = ChannelId::new(1);

        assert_eq!(tracker.record_message(ch), 1);
        assert_eq!(tracker.record_message(ch), 2);
        assert_eq!(tracker.record_message(ch), 3);
        assert_eq!(tracker.count(ch), 3);
    }

    #[test]
    fn different_channels_independent() {
        let tracker = SlowmodeTracker::new(60);
        let ch_a = ChannelId::new(1);
        let ch_b = ChannelId::new(2);

        tracker.record_message(ch_a);
        tracker.record_message(ch_a);
        tracker.record_message(ch_b);

        assert_eq!(tracker.count(ch_a), 2);
        assert_eq!(tracker.count(ch_b), 1);
    }

    #[test]
    fn should_activate_below_threshold() {
        let tracker = SlowmodeTracker::new(60);
        let ch = ChannelId::new(1);

        tracker.record_message(ch);
        tracker.record_message(ch);
        assert!(!tracker.should_activate(ch, 5));
    }

    #[test]
    fn should_activate_at_threshold() {
        let tracker = SlowmodeTracker::new(60);
        let ch = ChannelId::new(1);

        for _ in 0..5 {
            tracker.record_message(ch);
        }
        assert!(tracker.should_activate(ch, 5));
    }

    #[test]
    fn should_activate_empty_channel() {
        let tracker = SlowmodeTracker::new(60);
        assert!(!tracker.should_activate(ChannelId::new(1), 5));
    }

    #[test]
    fn reset_clears() {
        let tracker = SlowmodeTracker::new(60);
        let ch = ChannelId::new(1);

        tracker.record_message(ch);
        tracker.record_message(ch);
        tracker.reset(ch);
        assert_eq!(tracker.count(ch), 0);
    }
}
