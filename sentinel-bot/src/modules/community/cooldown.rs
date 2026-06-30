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

        // Cleanup inline : si la map devient grosse, retirer les entrees
        // dont le cooldown maximal (60s suffisent pour tous les cas) est
        // deja expire. Evite un leak long-terme. Fait AVANT le `entry`
        // ci-dessous : `retain` verrouille tous les shards, l'appeler
        // pendant qu'on tient le lock d'une entry risquerait un deadlock.
        if self.map.len() > 1000 {
            self.map.retain(|_, ts| ts.elapsed().as_secs() < 60);
        }

        // Atomicite : check-then-set en une seule operation verrouillee via
        // l'API `entry` de DashMap. Le shard de la cle reste verrouille entre
        // la lecture du timestamp et l'ecriture, donc deux interactions
        // concurrentes sur la meme cle ne peuvent plus passer toutes les deux
        // (TOCTOU corrige).
        use dashmap::mapref::entry::Entry;
        match self.map.entry(k) {
            Entry::Occupied(mut e) => {
                let elapsed = e.get().elapsed().as_secs();
                if elapsed < cooldown_secs {
                    return Some(cooldown_secs - elapsed);
                }
                e.insert(now);
                None
            }
            Entry::Vacant(e) => {
                e.insert(now);
                None
            }
        }
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
