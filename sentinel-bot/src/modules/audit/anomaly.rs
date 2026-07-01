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
/// Les seuils sont passes a chaque record() pour permettre un override
/// per-guild (lus depuis bot_guild_config par les handlers).
pub struct AnomalyDetector {
    counters: DashMap<(GuildId, String), Vec<Instant>>,
    window: Duration,
    default_thresholds: AnomalyThresholds,
    /// Taille max du buffer d'horodatages par (guild, categorie) avant eviction.
    max_buffer_size: usize,
    /// Nombre d'horodatages conserves apres eviction (les plus recents).
    eviction_target: usize,
}

impl AnomalyDetector {
    pub fn new(
        window_secs: u64,
        default_thresholds: AnomalyThresholds,
        max_buffer_size: usize,
        eviction_target: usize,
    ) -> Self {
        // Garde-fous : la cible d'eviction doit valoir au moins 1 et ne jamais
        // depasser la taille max du buffer (sinon la logique de drain panique
        // ou ne libere rien).
        let eviction_target = eviction_target.max(1);
        let max_buffer_size = max_buffer_size.max(eviction_target);
        Self {
            counters: DashMap::new(),
            window: Duration::from_secs(window_secs),
            default_thresholds,
            max_buffer_size,
            eviction_target,
        }
    }

    /// Enregistre un evenement et retourne une alerte si le seuil est atteint.
    /// Categories : "ban", "delete", "role_change", "kick"
    /// `thresholds` : si None, utilise les seuils par defaut du detecteur.
    pub fn record(
        &self,
        guild_id: GuildId,
        category: &str,
        thresholds: Option<&AnomalyThresholds>,
    ) -> Option<AnomalyAlert> {
        let now = Instant::now();
        let key = (guild_id, category.to_string());
        let mut entry = self.counters.entry(key).or_default();
        let timestamps = entry.value_mut();

        // Nettoyer hors fenetre
        timestamps.retain(|t| now.duration_since(*t) < self.window);
        // Securite : limiter la taille du vecteur
        if timestamps.len() > self.max_buffer_size {
            timestamps.drain(0..timestamps.len() - self.eviction_target);
        }
        timestamps.push(now);

        let count = timestamps.len();
        let effective = thresholds.unwrap_or(&self.default_thresholds);
        let threshold = match category {
            "ban" | "kick" => effective.mass_ban,
            "delete" => effective.mass_delete,
            "role_change" => effective.mass_role_change,
            _ => usize::MAX,
        };

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
            500,
            100,
        )
    }

    #[test]
    fn eviction_guard_clamps_target_to_buffer() {
        // eviction_target > max_buffer_size doit etre borne a max_buffer_size.
        let detector = AnomalyDetector::new(60, AnomalyThresholds::default(), 50, 200);
        assert_eq!(detector.max_buffer_size, 200);
        assert_eq!(detector.eviction_target, 200);
    }

    #[test]
    fn eviction_guard_target_at_least_one() {
        let detector = AnomalyDetector::new(60, AnomalyThresholds::default(), 500, 0);
        assert_eq!(detector.eviction_target, 1);
        assert_eq!(detector.max_buffer_size, 500);
    }

    #[test]
    fn no_alert_below_threshold() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        assert!(detector.record(guild, "ban", None).is_none());
        assert!(detector.record(guild, "ban", None).is_none());
    }

    #[test]
    fn alert_at_threshold() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        assert!(detector.record(guild, "ban", None).is_none());
        assert!(detector.record(guild, "ban", None).is_none());
        let alert = detector.record(guild, "ban", None);
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
        detector.record(guild, "ban", None);
        detector.record(guild, "ban", None);
        assert!(detector.record(guild, "ban", None).is_some());

        // Apres reset, il faut a nouveau atteindre le seuil
        assert!(detector.record(guild, "ban", None).is_none());
        assert!(detector.record(guild, "ban", None).is_none());
        assert!(detector.record(guild, "ban", None).is_some());
    }

    #[test]
    fn different_categories_independent() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        detector.record(guild, "ban", None);
        detector.record(guild, "ban", None);
        detector.record(guild, "delete", None);
        detector.record(guild, "delete", None);

        // Ban n'a pas encore atteint son seuil (2/3)
        // Delete non plus (2/5)
        // Pas d'alerte croisee
    }

    #[test]
    fn different_guilds_independent() {
        let detector = make_detector();
        let guild_a = GuildId::new(1);
        let guild_b = GuildId::new(2);

        detector.record(guild_a, "ban", None);
        detector.record(guild_a, "ban", None);
        detector.record(guild_b, "ban", None);

        // Guild A a 2 bans, guild B a 1 — aucune alerte
        assert!(detector.record(guild_b, "ban", None).is_none()); // B = 2
        assert!(detector.record(guild_a, "ban", None).is_some()); // A = 3 -> alerte
    }

    #[test]
    fn delete_threshold_different() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        for _ in 0..4 {
            assert!(detector.record(guild, "delete", None).is_none());
        }
        assert!(detector.record(guild, "delete", None).is_some());
    }

    #[test]
    fn role_change_threshold() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        for _ in 0..3 {
            assert!(detector.record(guild, "role_change", None).is_none());
        }
        assert!(detector.record(guild, "role_change", None).is_some());
    }

    #[test]
    fn kick_uses_ban_threshold() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        detector.record(guild, "kick", None);
        detector.record(guild, "kick", None);
        assert!(detector.record(guild, "kick", None).is_some());
    }

    #[test]
    fn unknown_category_never_alerts() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        for _ in 0..100 {
            assert!(detector.record(guild, "unknown", None).is_none());
        }
    }
}
