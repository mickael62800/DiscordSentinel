use std::time::{Duration, Instant};

use dashmap::{DashMap, DashSet};
use serenity::model::id::ChannelId;

/// Tracker d'activite par channel pour le slowmode adaptatif.
pub struct SlowmodeTracker {
    /// channel_id -> timestamps des messages recents
    counters: DashMap<ChannelId, Vec<Instant>>,
    /// channels en cours d'activation (evite les activations multiples)
    activating: DashSet<ChannelId>,
    window: Duration,
}

impl SlowmodeTracker {
    pub fn new(window_secs: u64) -> Self {
        Self {
            counters: DashMap::new(),
            activating: DashSet::new(),
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

    /// Tente de demarrer l'activation du slowmode. Retourne true si ok (pas deja en cours).
    pub fn try_start_activation(&self, channel_id: ChannelId) -> bool {
        self.activating.insert(channel_id)
    }

    /// Termine l'activation du slowmode.
    pub fn finish_activation(&self, channel_id: ChannelId) {
        self.activating.remove(&channel_id);
    }

    /// Reset le compteur d'un channel.
    #[allow(dead_code)]
    pub fn reset(&self, channel_id: ChannelId) {
        self.counters.remove(&channel_id);
    }

    /// Supprime les channels inactifs (pas de message depuis > 2x la fenetre).
    pub fn cleanup(&self) {
        let now = Instant::now();
        let max_age = self.window * 2;
        self.counters.retain(|_, ts| {
            ts.last()
                .map(|t| now.duration_since(*t) < max_age)
                .unwrap_or(false)
        });
    }

    /// Retourne le nombre de channels actuellement suivis.
    pub fn tracked_channels(&self) -> usize {
        self.counters.len()
    }

    /// Retourne les channels dont le slowmode a ete active mais dont l'activite
    /// est retombee sous le seuil (aucun message dans la fenetre).
    /// Ces channels devraient avoir leur slowmode desactive.
    pub fn channels_to_deactivate(&self, threshold: usize) -> Vec<ChannelId> {
        let now = Instant::now();
        self.counters
            .iter()
            .filter(|entry| {
                let active_count = entry
                    .value()
                    .iter()
                    .filter(|t| now.duration_since(**t) < self.window)
                    .count();
                // Si l'activite est retombee sous la moitie du seuil, desactiver
                active_count < threshold / 2
            })
            .map(|entry| *entry.key())
            .collect()
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
    fn try_start_activation_returns_true_first_time() {
        let tracker = SlowmodeTracker::new(60);
        let ch = ChannelId::new(1);
        assert!(tracker.try_start_activation(ch));
    }

    #[test]
    fn try_start_activation_returns_false_if_already_activating() {
        let tracker = SlowmodeTracker::new(60);
        let ch = ChannelId::new(1);
        assert!(tracker.try_start_activation(ch));
        assert!(!tracker.try_start_activation(ch)); // deja en cours
    }

    #[test]
    fn finish_activation_allows_reactivation() {
        let tracker = SlowmodeTracker::new(60);
        let ch = ChannelId::new(1);
        tracker.try_start_activation(ch);
        tracker.finish_activation(ch);
        assert!(tracker.try_start_activation(ch)); // de nouveau possible
    }

    #[test]
    fn tracked_channels_count() {
        let tracker = SlowmodeTracker::new(60);
        assert_eq!(tracker.tracked_channels(), 0);
        tracker.record_message(ChannelId::new(1));
        tracker.record_message(ChannelId::new(2));
        assert_eq!(tracker.tracked_channels(), 2);
    }

    #[test]
    fn cleanup_removes_old_channels() {
        let tracker = SlowmodeTracker::new(1); // window 1s
        let ch = ChannelId::new(1);
        tracker.record_message(ch);
        // Forcer l'entree a etre vieille (on insere directement)
        tracker.counters.entry(ch).and_modify(|ts| {
            ts.clear();
            ts.push(Instant::now() - Duration::from_secs(10));
        });
        tracker.cleanup();
        assert_eq!(
            tracker.tracked_channels(),
            0,
            "Le channel inactif doit etre nettoye"
        );
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

    // ── Tests pour channels_to_deactivate ──

    #[test]
    fn deactivate_empty_tracker() {
        let tracker = SlowmodeTracker::new(60);
        assert!(tracker.channels_to_deactivate(10).is_empty());
    }

    #[test]
    fn deactivate_returns_quiet_channels() {
        let tracker = SlowmodeTracker::new(60);
        let ch = ChannelId::new(1);
        // 2 messages = sous la moitie de 10 (seuil/2 = 5)
        tracker.record_message(ch);
        tracker.record_message(ch);
        let to_deactivate = tracker.channels_to_deactivate(10);
        assert_eq!(to_deactivate.len(), 1);
        assert_eq!(to_deactivate[0], ch);
    }

    #[test]
    fn deactivate_ignores_active_channels() {
        let tracker = SlowmodeTracker::new(60);
        let ch = ChannelId::new(1);
        // 8 messages = au-dessus de seuil/2 = 5
        for _ in 0..8 {
            tracker.record_message(ch);
        }
        assert!(tracker.channels_to_deactivate(10).is_empty());
    }

    #[test]
    fn deactivate_mixed_channels() {
        let tracker = SlowmodeTracker::new(60);
        let quiet = ChannelId::new(1);
        let active = ChannelId::new(2);
        tracker.record_message(quiet);
        for _ in 0..8 {
            tracker.record_message(active);
        }
        let to_deactivate = tracker.channels_to_deactivate(10);
        assert_eq!(to_deactivate.len(), 1);
        assert_eq!(to_deactivate[0], quiet);
    }
}
