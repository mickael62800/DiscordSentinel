use std::time::{Duration, Instant};

use dashmap::DashMap;

/// Role temporaire avec expiration.
#[derive(Debug, Clone)]
pub struct TempRole {
    pub guild_id: u64,
    pub user_id: u64,
    pub role_id: u64,
}

/// Tracker de roles temporaires.
pub struct TempRoleTracker {
    /// (guild_id, user_id, role_id) → expiry instant
    roles: DashMap<(u64, u64, u64), Instant>,
}

impl TempRoleTracker {
    pub fn new() -> Self {
        Self {
            roles: DashMap::new(),
        }
    }

    /// Ajoute un role temporaire avec une duree d'expiration.
    pub fn add(&self, guild_id: u64, user_id: u64, role_id: u64, duration_secs: u64) {
        let expiry = Instant::now() + Duration::from_secs(duration_secs);
        self.roles.insert((guild_id, user_id, role_id), expiry);
    }

    /// Retourne les roles expires.
    pub fn expired(&self) -> Vec<TempRole> {
        let now = Instant::now();
        self.roles
            .iter()
            .filter(|entry| now >= *entry.value())
            .map(|entry| {
                let (guild_id, user_id, role_id) = *entry.key();
                TempRole { guild_id, user_id, role_id }
            })
            .collect()
    }

    /// Supprime un role du tracking.
    pub fn remove(&self, guild_id: u64, user_id: u64, role_id: u64) {
        self.roles.remove(&(guild_id, user_id, role_id));
    }

    /// Verifie si un role est temporaire pour cet utilisateur.
    #[allow(dead_code)]
    pub fn is_temp(&self, guild_id: u64, user_id: u64, role_id: u64) -> bool {
        self.roles.contains_key(&(guild_id, user_id, role_id))
    }
}

/// Parse les roles temporaires depuis la config : "role_id:duration_secs" par ligne.
pub fn parse_temp_roles(raw: &str) -> Vec<(u64, u64)> {
    sentinel_shared::parsers::parse_id_u64_lines(raw)
        .into_iter()
        .filter(|(_, dur)| *dur > 0)
        .collect()
}

/// Retourne la duree temporaire pour un role, ou None si non temporaire.
pub fn get_temp_duration(temp_roles: &[(u64, u64)], role_id: u64) -> Option<u64> {
    sentinel_shared::parsers::lookup_u64(temp_roles, role_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_check() {
        let tracker = TempRoleTracker::new();
        tracker.add(1, 100, 200, 3600);
        assert!(tracker.is_temp(1, 100, 200));
        assert!(!tracker.is_temp(1, 100, 999));
    }

    #[test]
    fn expired_returns_expired() {
        let tracker = TempRoleTracker::new();
        // Ajouter avec duree 0 (expire immediatement)
        tracker.roles.insert((1, 100, 200), Instant::now() - Duration::from_secs(10));
        let expired = tracker.expired();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].role_id, 200);
    }

    #[test]
    fn expired_ignores_active() {
        let tracker = TempRoleTracker::new();
        tracker.add(1, 100, 200, 3600); // expire dans 1h
        assert!(tracker.expired().is_empty());
    }

    #[test]
    fn remove_cleans_up() {
        let tracker = TempRoleTracker::new();
        tracker.add(1, 100, 200, 3600);
        tracker.remove(1, 100, 200);
        assert!(!tracker.is_temp(1, 100, 200));
    }

    #[test]
    fn parse_temp_roles_simple() {
        let raw = "111:3600\n222:86400";
        let roles = parse_temp_roles(raw);
        assert_eq!(roles, vec![(111, 3600), (222, 86400)]);
    }

    #[test]
    fn parse_temp_roles_empty() {
        assert!(parse_temp_roles("").is_empty());
    }

    #[test]
    fn parse_temp_roles_ignores_invalid() {
        let raw = "abc:100\n111:0\n222:3600";
        let roles = parse_temp_roles(raw);
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], (222, 3600));
    }

    #[test]
    fn get_temp_duration_found() {
        let roles = vec![(111, 3600), (222, 86400)];
        assert_eq!(get_temp_duration(&roles, 111), Some(3600));
    }

    #[test]
    fn get_temp_duration_not_found() {
        let roles = vec![(111, 3600)];
        assert_eq!(get_temp_duration(&roles, 999), None);
    }
}
