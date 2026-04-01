use std::time::{Duration, Instant};

use dashmap::DashMap;
use serenity::model::id::GuildId;

/// Seuils de detection d'anomalie.
#[derive(Debug, Clone)]
pub struct AnomalyThresholds {
    pub mass_ban: usize,
    pub mass_delete: usize,
    pub mass_role_change: usize,
}

impl Default for AnomalyThresholds {
    fn default() -> Self {
        Self {
            mass_ban: 5,
            mass_delete: 20,
            mass_role_change: 10,
        }
    }
}

/// Alerte d'anomalie detectee.
#[derive(Debug, Clone)]
pub struct AnomalyAlert {
    pub anomaly_type: String,
    pub count: usize,
    pub window_secs: u64,
}

/// Detecteur d'anomalies base sur des compteurs a fenetre glissante.
pub struct AnomalyDetector {
    counters: DashMap<(GuildId, String), Vec<Instant>>,
    window: Duration,
    thresholds: AnomalyThresholds,
}

impl AnomalyDetector {
    pub fn new(window_secs: u64, thresholds: AnomalyThresholds) -> Self {
        Self {
            counters: DashMap::new(),
            window: Duration::from_secs(window_secs),
            thresholds,
        }
    }

    /// Enregistre un evenement et retourne une alerte si le seuil est atteint.
    /// Categories : "ban", "delete", "role_change", "kick"
    pub fn record(&self, guild_id: GuildId, category: &str) -> Option<AnomalyAlert> {
        let now = Instant::now();
        let key = (guild_id, category.to_string());
        let mut entry = self.counters.entry(key).or_default();
        let timestamps = entry.value_mut();

        // Nettoyer hors fenetre
        timestamps.retain(|t| now.duration_since(*t) < self.window);
        timestamps.push(now);

        let count = timestamps.len();
        let threshold = self.threshold_for(category);

        if count >= threshold {
            // Reset pour eviter les alertes en boucle
            timestamps.clear();

            Some(AnomalyAlert {
                anomaly_type: format!("mass_{}", category),
                count,
                window_secs: self.window.as_secs(),
            })
        } else {
            None
        }
    }

    fn threshold_for(&self, category: &str) -> usize {
        match category {
            "ban" | "kick" => self.thresholds.mass_ban,
            "delete" => self.thresholds.mass_delete,
            "role_change" => self.thresholds.mass_role_change,
            _ => usize::MAX,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_detector() -> AnomalyDetector {
        AnomalyDetector::new(
            60,
            AnomalyThresholds {
                mass_ban: 3,
                mass_delete: 5,
                mass_role_change: 4,
            },
        )
    }

    #[test]
    fn no_alert_below_threshold() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        assert!(detector.record(guild, "ban").is_none());
        assert!(detector.record(guild, "ban").is_none());
    }

    #[test]
    fn alert_at_threshold() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        assert!(detector.record(guild, "ban").is_none());
        assert!(detector.record(guild, "ban").is_none());
        let alert = detector.record(guild, "ban");
        assert!(alert.is_some());

        let alert = alert.unwrap();
        assert_eq!(alert.anomaly_type, "mass_ban");
        assert_eq!(alert.count, 3);
        assert_eq!(alert.window_secs, 60);
    }

    #[test]
    fn reset_after_alert() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        // Declencher l'alerte
        detector.record(guild, "ban");
        detector.record(guild, "ban");
        assert!(detector.record(guild, "ban").is_some());

        // Apres reset, il faut a nouveau atteindre le seuil
        assert!(detector.record(guild, "ban").is_none());
        assert!(detector.record(guild, "ban").is_none());
        assert!(detector.record(guild, "ban").is_some());
    }

    #[test]
    fn different_categories_independent() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        detector.record(guild, "ban");
        detector.record(guild, "ban");
        detector.record(guild, "delete");
        detector.record(guild, "delete");

        // Ban n'a pas encore atteint son seuil (2/3)
        // Delete non plus (2/5)
        // Pas d'alerte croisee
    }

    #[test]
    fn different_guilds_independent() {
        let detector = make_detector();
        let guild_a = GuildId::new(1);
        let guild_b = GuildId::new(2);

        detector.record(guild_a, "ban");
        detector.record(guild_a, "ban");
        detector.record(guild_b, "ban");

        // Guild A a 2 bans, guild B a 1 — aucune alerte
        assert!(detector.record(guild_b, "ban").is_none()); // B = 2
        assert!(detector.record(guild_a, "ban").is_some()); // A = 3 → alerte
    }

    #[test]
    fn delete_threshold_different() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        for _ in 0..4 {
            assert!(detector.record(guild, "delete").is_none());
        }
        assert!(detector.record(guild, "delete").is_some());
    }

    #[test]
    fn role_change_threshold() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        for _ in 0..3 {
            assert!(detector.record(guild, "role_change").is_none());
        }
        assert!(detector.record(guild, "role_change").is_some());
    }

    #[test]
    fn kick_uses_ban_threshold() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        detector.record(guild, "kick");
        detector.record(guild, "kick");
        assert!(detector.record(guild, "kick").is_some());
    }

    #[test]
    fn unknown_category_never_alerts() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        for _ in 0..100 {
            assert!(detector.record(guild, "unknown").is_none());
        }
    }
}
