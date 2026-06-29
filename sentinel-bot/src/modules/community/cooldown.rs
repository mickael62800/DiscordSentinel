//! Rate limiter per-user pour eviter le spam d'interactions (boutons, commandes).
//!
//! Utilise une DashMap (user_id, bucket_key) -> Instant. Le `bucket_key` est
//! un identifiant libre qui permet d'avoir plusieurs cooldowns distincts par
//! user (ex: "role_toggle", "parrain_command"). Cleanup inline dans `check()`
//! pour eviter le leak memoire.

use std::time::Instant;

use dashmap::DashMap;

/// Bucket = (user_id, key) -> dernier timestamp de trigger.
pub struct InteractionCooldown {
    map: DashMap<(u64, String), Instant>,
}

impl InteractionCooldown {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    /// Verifie le cooldown. Retourne `Some(remaining_secs)` si le user doit
    /// encore attendre, `None` si l'action est autorisee (et alors enregistre
    /// le nouveau timestamp).
    pub fn check_and_set(&self, user_id: u64, key: &str, cooldown_secs: u64) -> Option<u64> {
        let k = (user_id, key.to_string());
        let now = Instant::now();

        if let Some(entry) = self.map.get(&k) {
            let elapsed = entry.value().elapsed().as_secs();
            if elapsed < cooldown_secs {
                return Some(cooldown_secs - elapsed);
            }
        }

        // Cleanup inline : si la map devient grosse, retirer les entrees
        // dont le cooldown maximal (60s suffisent pour tous les cas) est
        // deja expire. Evite un leak long-terme.
        if self.map.len() > 1000 {
            self.map.retain(|_, ts| ts.elapsed().as_secs() < 60);
        }

        self.map.insert(k, now);
        None
    }
}

impl Default for InteractionCooldown {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn first_call_allowed() {
        let c = InteractionCooldown::new();
        assert_eq!(c.check_and_set(1, "role", 5), None);
    }

    #[test]
    fn second_call_blocked() {
        let c = InteractionCooldown::new();
        c.check_and_set(1, "role", 5);
        let result = c.check_and_set(1, "role", 5);
        assert!(result.is_some());
        assert!(result.unwrap() <= 5);
    }

    #[test]
    fn different_keys_independent() {
        let c = InteractionCooldown::new();
        c.check_and_set(1, "role", 5);
        // Meme user, clé differente → pas de cooldown
        assert_eq!(c.check_and_set(1, "parrain", 5), None);
    }

    #[test]
    fn different_users_independent() {
        let c = InteractionCooldown::new();
        c.check_and_set(1, "role", 5);
        assert_eq!(c.check_and_set(2, "role", 5), None);
    }

    #[test]
    fn cooldown_expires() {
        let c = InteractionCooldown::new();
        c.check_and_set(1, "role", 1);
        sleep(Duration::from_millis(1100));
        assert_eq!(c.check_and_set(1, "role", 1), None);
    }
}
