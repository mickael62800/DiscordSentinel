use std::time::{Duration, Instant};

use dashmap::DashMap;

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

    /// Ajoute un role temporaire avec un timestamp d'expiration absolu (pour le reload depuis l'API).
    pub fn add_with_expiry_timestamp(
        &self,
        guild_id: u64,
        user_id: u64,
        role_id: u64,
        expires_at: &str,
    ) {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(expires_at) {
            let now = chrono::Utc::now();
            let remaining = dt.signed_duration_since(now);
            if remaining.num_seconds() > 0 {
                let expiry = Instant::now() + Duration::from_secs(remaining.num_seconds() as u64);
                self.roles.insert((guild_id, user_id, role_id), expiry);
            }
        }
    }

    /// Supprime un role du tracking.
    /// Accesseur de test : la production n'interroge jamais le tracker, elle
    /// ne fait qu'y ajouter et retirer. Il existe pour que les tests puissent
    /// verifier `add`, `remove` et `add_with_expiry_timestamp` — sans lui,
    /// ces trois methodes ne seraient plus couvertes.
    #[cfg(test)]
    pub fn is_temp(&self, guild_id: u64, user_id: u64, role_id: u64) -> bool {
        self.roles.contains_key(&(guild_id, user_id, role_id))
    }

    pub fn remove(&self, guild_id: u64, user_id: u64, role_id: u64) {
        self.roles.remove(&(guild_id, user_id, role_id));
    }
}

/// Parse les roles temporaires depuis la config : "role_id:duration_secs" par ligne.
pub fn parse_temp_roles(raw: &str) -> Vec<(u64, u64)> {
    crate::shared::parsers::parse_id_u64_lines(raw)
        .into_iter()
        .filter(|(_, dur)| *dur > 0)
        .collect()
}

/// Retourne la duree temporaire pour un role, ou None si non temporaire.
pub fn get_temp_duration(temp_roles: &[(u64, u64)], role_id: u64) -> Option<u64> {
    crate::shared::parsers::lookup_u64(temp_roles, role_id)
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
    fn add_with_expiry_timestamp_future() {
        let tracker = TempRoleTracker::new();
        let future = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        tracker.add_with_expiry_timestamp(1, 100, 200, &future);
        assert!(tracker.is_temp(1, 100, 200));
    }

    #[test]
    fn add_with_expiry_timestamp_past_ignored() {
        let tracker = TempRoleTracker::new();
        let past = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        tracker.add_with_expiry_timestamp(1, 100, 200, &past);
        assert!(!tracker.is_temp(1, 100, 200)); // pas insere car deja expire
    }

    #[test]
    fn add_with_expiry_timestamp_invalid_format() {
        let tracker = TempRoleTracker::new();
        tracker.add_with_expiry_timestamp(1, 100, 200, "not-a-date");
        assert!(!tracker.is_temp(1, 100, 200)); // pas insere
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
