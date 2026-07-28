//! Cooldown par clé avec check-and-set atomique — le mécanisme commun de
//! `voice::CooldownTracker` et `community::InteractionCooldown`.
//!
//! Atomicité : check-then-set en une seule opération verrouillée via l'API
//! `entry` de DashMap. Le shard de la clé reste verrouillé entre la lecture du
//! timestamp et l'écriture, donc deux évènements concurrents sur la même clé
//! ne peuvent pas passer tous les deux (TOCTOU corrigé).
//!
//! Purge UNIQUE : amortie dans `check_and_set`, quand la map dépasse
//! `max_entries`, on retire les entrées dont le cooldown courant est expiré.
//! Fait AVANT le `entry` (retain verrouille tous les shards, l'appeler en
//! tenant le lock d'une entry risquerait un deadlock).

use std::hash::Hash;
use std::time::Instant;

use dashmap::DashMap;

pub struct CooldownMap<K: Eq + Hash> {
    map: DashMap<K, Instant>,
    max_entries: usize,
}

impl<K: Eq + Hash> CooldownMap<K> {
    pub fn new(max_entries: usize) -> Self {
        Self {
            map: DashMap::new(),
            max_entries,
        }
    }

    /// Vérifie ET pose le cooldown de manière atomique. Retourne
    /// `Some(remaining_secs)` si la clé est encore en cooldown (rien n'est
    /// écrit), `None` si l'action est autorisée (le timestamp est alors
    /// enregistré).
    pub fn check_and_set(&self, key: K, cooldown_secs: u64) -> Option<u64> {
        let now = Instant::now();

        if self.map.len() > self.max_entries {
            self.map
                .retain(|_, ts| ts.elapsed().as_secs() < cooldown_secs);
        }

        use dashmap::mapref::entry::Entry;
        match self.map.entry(key) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_call_allowed_second_blocked() {
        let c = CooldownMap::<u64>::new(1000);
        assert_eq!(c.check_and_set(1, 5), None);
        let remaining = c.check_and_set(1, 5);
        assert!(remaining.is_some());
        assert!(remaining.unwrap() <= 5);
    }

    #[test]
    fn keys_independent() {
        let c = CooldownMap::<u64>::new(1000);
        c.check_and_set(1, 5);
        assert_eq!(c.check_and_set(2, 5), None);
    }

    #[test]
    fn cooldown_expires() {
        let c = CooldownMap::<u64>::new(1000);
        c.check_and_set(1, 1);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert_eq!(c.check_and_set(1, 1), None);
    }
}
