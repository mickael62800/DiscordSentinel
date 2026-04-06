use std::time::Instant;

use dashmap::DashMap;
use serenity::model::id::GuildId;

/// Détecteur de raid basé sur le nombre de joins dans une fenêtre de temps.
/// Thread-safe grâce à DashMap.
pub struct RaidDetector {
    /// guild_id → liste des timestamps de join
    joins: DashMap<GuildId, Vec<Instant>>,
    threshold: u64,
    window: std::time::Duration,
}

impl RaidDetector {
    pub fn new(threshold: u64, window_secs: u64) -> Self {
        Self {
            joins: DashMap::new(),
            threshold,
            window: std::time::Duration::from_secs(window_secs),
        }
    }

    /// Enregistre un join et retourne `true` si un raid est détecté.
    pub fn record_join(&self, guild_id: GuildId) -> bool {
        let now = Instant::now();
        let mut entry = self.joins.entry(guild_id).or_default();
        let timestamps = entry.value_mut();

        // Nettoyer les joins hors fenêtre
        timestamps.retain(|t| now.duration_since(*t) < self.window);

        // Ajouter le nouveau join
        timestamps.push(now);

        let result = timestamps.len() as u64 >= self.threshold;
        drop(entry);

        // Cleanup periodique : supprimer les guildes avec vecteur vide
        if self.joins.len() > 1000 {
            self.joins.retain(|_, ts| !ts.is_empty());
        }

        result
    }

    /// Retourne le nombre de joins récents pour un guild.
    pub fn recent_joins(&self, guild_id: GuildId) -> u64 {
        let now = Instant::now();
        self.joins
            .get(&guild_id)
            .map(|entry| {
                entry
                    .value()
                    .iter()
                    .filter(|t| now.duration_since(**t) < self.window)
                    .count() as u64
            })
            .unwrap_or(0)
    }

    /// Réinitialise les compteurs d'un guild (après lockdown par ex).
    pub fn reset(&self, guild_id: GuildId) {
        self.joins.remove(&guild_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_raid_below_threshold() {
        let detector = RaidDetector::new(5, 10);
        let guild = GuildId::new(1);
        for _ in 0..4 {
            assert!(!detector.record_join(guild));
        }
    }

    #[test]
    fn test_raid_at_threshold() {
        let detector = RaidDetector::new(3, 10);
        let guild = GuildId::new(1);
        assert!(!detector.record_join(guild));
        assert!(!detector.record_join(guild));
        assert!(detector.record_join(guild)); // 3ème = raid
    }

    #[test]
    fn test_different_guilds_independent() {
        let detector = RaidDetector::new(2, 10);
        let guild_a = GuildId::new(1);
        let guild_b = GuildId::new(2);
        assert!(!detector.record_join(guild_a));
        assert!(!detector.record_join(guild_b));
        assert!(detector.record_join(guild_a)); // 2ème pour A
        assert!(detector.record_join(guild_b)); // 2ème pour B
    }

    #[test]
    fn test_reset_clears_count() {
        let detector = RaidDetector::new(3, 10);
        let guild = GuildId::new(1);
        detector.record_join(guild);
        detector.record_join(guild);
        detector.reset(guild);
        assert_eq!(detector.recent_joins(guild), 0);
        assert!(!detector.record_join(guild)); // repart de 1
    }
}
