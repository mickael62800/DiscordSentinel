#![allow(dead_code)]
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serenity::model::id::GuildId;

use super::raid_analyzer::levenshtein;

/// Enregistrement d'un ban/kick recent.
#[derive(Clone, Debug)]
pub struct BanRecord {
    pub username: String,
    pub account_created_timestamp: i64,
    pub recorded_at: Instant,
}

/// Resultat de l'analyse alt-account.
#[derive(Debug)]
pub struct AltAnalysis {
    /// Nom du banni dont le pseudo est similaire.
    pub similar_to_banned: Option<String>,
    /// Nom du banni dont la date de creation est proche.
    pub creation_near_banned: Option<String>,
}

impl AltAnalysis {
    pub fn is_suspicious(&self) -> bool {
        self.similar_to_banned.is_some() || self.creation_near_banned.is_some()
    }
}

/// Detecteur de comptes alt bases sur les bans/kicks recents.
pub struct AltDetector {
    recent_bans: DashMap<GuildId, Vec<BanRecord>>,
    retention: Duration,
    name_distance_threshold: usize,
    creation_cluster_secs: i64,
}

impl AltDetector {
    pub fn new(retention_secs: u64, name_distance: usize, cluster_secs: i64) -> Self {
        Self {
            recent_bans: DashMap::new(),
            retention: Duration::from_secs(retention_secs),
            name_distance_threshold: name_distance,
            creation_cluster_secs: cluster_secs,
        }
    }

    /// Enregistre un ban ou kick.
    pub fn record_ban(&self, guild_id: GuildId, username: String, account_created_timestamp: i64) {
        let mut entry = self.recent_bans.entry(guild_id).or_default();
        let list = entry.value_mut();

        // Nettoyer les entrees expirees
        let now = Instant::now();
        list.retain(|r| now.duration_since(r.recorded_at) < self.retention);

        list.push(BanRecord {
            username,
            account_created_timestamp,
            recorded_at: now,
        });
    }

    /// Verifie un utilisateur qui rejoint contre les bans recents.
    pub fn check_user(
        &self,
        guild_id: GuildId,
        username: &str,
        account_created_timestamp: i64,
    ) -> AltAnalysis {
        let mut similar_to_banned = None;
        let mut creation_near_banned = None;

        let entry = match self.recent_bans.get(&guild_id) {
            Some(e) => e,
            None => {
                return AltAnalysis {
                    similar_to_banned: None,
                    creation_near_banned: None,
                };
            }
        };

        let now = Instant::now();
        let username_lower = username.to_lowercase();

        for record in entry.value().iter() {
            // Ignorer les entrees expirees
            if now.duration_since(record.recorded_at) >= self.retention {
                continue;
            }

            // Comparaison Levenshtein des noms
            let record_lower = record.username.to_lowercase();
            if levenshtein(&username_lower, &record_lower) <= self.name_distance_threshold {
                similar_to_banned = Some(record.username.clone());
            }

            // Proximite de date de creation
            let diff = (account_created_timestamp - record.account_created_timestamp).abs();
            if diff <= self.creation_cluster_secs {
                creation_near_banned = Some(record.username.clone());
            }

            // Pas besoin de continuer si les deux sont trouves
            if similar_to_banned.is_some() && creation_near_banned.is_some() {
                break;
            }
        }

        AltAnalysis {
            similar_to_banned,
            creation_near_banned,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_detector() -> AltDetector {
        AltDetector::new(
            600,  // 10 min retention
            2,    // distance threshold
            3600, // 1h cluster
        )
    }

    // ── record_ban + check_user ──

    #[test]
    fn no_bans_no_detection() {
        let detector = make_detector();
        let guild = GuildId::new(1);
        let result = detector.check_user(guild, "alice", 1_000_000);
        assert!(!result.is_suspicious());
    }

    #[test]
    fn similar_name_detected() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        detector.record_ban(guild, "toxic_user".to_string(), 1_000_000);

        // Nom similaire (distance 1)
        let result = detector.check_user(guild, "toxic_usar", 5_000_000);
        assert!(result.similar_to_banned.is_some());
        assert_eq!(result.similar_to_banned.unwrap(), "toxic_user");
    }

    #[test]
    fn exact_name_detected() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        detector.record_ban(guild, "badguy".to_string(), 1_000_000);

        let result = detector.check_user(guild, "badguy", 5_000_000);
        assert!(result.similar_to_banned.is_some());
    }

    #[test]
    fn similar_name_case_insensitive() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        detector.record_ban(guild, "ToxicUser".to_string(), 1_000_000);

        let result = detector.check_user(guild, "toxicuser", 5_000_000);
        assert!(result.similar_to_banned.is_some());
    }

    #[test]
    fn different_name_no_detection() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        detector.record_ban(guild, "toxic_user".to_string(), 1_000_000);

        let result = detector.check_user(guild, "alice", 5_000_000);
        assert!(result.similar_to_banned.is_none());
    }

    #[test]
    fn creation_near_banned_detected() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        detector.record_ban(guild, "banned_bot".to_string(), 1_000_000);

        // Compte cree a 1000 secondes du banni (< 3600)
        let result = detector.check_user(guild, "totally_different_name", 1_001_000);
        assert!(result.creation_near_banned.is_some());
        assert_eq!(result.creation_near_banned.unwrap(), "banned_bot");
    }

    #[test]
    fn creation_far_from_banned_no_detection() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        detector.record_ban(guild, "banned_bot".to_string(), 1_000_000);

        let result = detector.check_user(guild, "new_user", 2_000_000);
        assert!(result.creation_near_banned.is_none());
    }

    #[test]
    fn both_signals_detected() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        detector.record_ban(guild, "raider01".to_string(), 1_000_000);

        // Nom similaire ET creation proche
        let result = detector.check_user(guild, "raider02", 1_000_500);
        assert!(result.similar_to_banned.is_some());
        assert!(result.creation_near_banned.is_some());
        assert!(result.is_suspicious());
    }

    #[test]
    fn different_guilds_independent() {
        let detector = make_detector();
        let guild_a = GuildId::new(1);
        let guild_b = GuildId::new(2);

        detector.record_ban(guild_a, "toxic".to_string(), 1_000_000);

        // Pas de detection sur guild_b
        let result = detector.check_user(guild_b, "toxic", 1_000_000);
        assert!(!result.is_suspicious());
    }

    #[test]
    fn multiple_bans_one_match() {
        let detector = make_detector();
        let guild = GuildId::new(1);

        detector.record_ban(guild, "alice".to_string(), 1_000_000);
        detector.record_ban(guild, "bob".to_string(), 2_000_000);
        detector.record_ban(guild, "raider".to_string(), 3_000_000);

        let result = detector.check_user(guild, "raider2", 5_000_000);
        assert!(result.similar_to_banned.is_some());
        assert_eq!(result.similar_to_banned.unwrap(), "raider");
    }

    // ── AltAnalysis ──

    #[test]
    fn alt_analysis_not_suspicious() {
        let a = AltAnalysis {
            similar_to_banned: None,
            creation_near_banned: None,
        };
        assert!(!a.is_suspicious());
    }

    #[test]
    fn alt_analysis_suspicious_name() {
        let a = AltAnalysis {
            similar_to_banned: Some("toxic".to_string()),
            creation_near_banned: None,
        };
        assert!(a.is_suspicious());
    }

    #[test]
    fn alt_analysis_suspicious_creation() {
        let a = AltAnalysis {
            similar_to_banned: None,
            creation_near_banned: Some("toxic".to_string()),
        };
        assert!(a.is_suspicious());
    }
}
