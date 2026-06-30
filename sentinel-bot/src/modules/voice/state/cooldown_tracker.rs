use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use dashmap::DashMap;
use serenity::model::id::UserId;

const DEFAULT_COOLDOWN_SECS: u64 = 5;

pub struct CooldownTracker {
    map: DashMap<UserId, Instant>,
    cooldown_secs: AtomicU64,
}

impl CooldownTracker {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            cooldown_secs: AtomicU64::new(DEFAULT_COOLDOWN_SECS),
        }
    }

    /// Met a jour le cooldown depuis la config API.
    pub fn set_cooldown_secs(&self, secs: u64) {
        self.cooldown_secs.store(secs, Ordering::Relaxed);
    }

    fn cooldown(&self) -> u64 {
        self.cooldown_secs.load(Ordering::Relaxed)
    }

    /// Verifie ET pose le cooldown de maniere atomique. Retourne
    /// `Some(remaining_secs)` si l'utilisateur est encore en cooldown (rien
    /// n'est ecrit), `None` si l'action est autorisee (le timestamp est alors
    /// enregistre).
    ///
    /// A privilegier sur `check` + `set` separes : ces deux appels formaient un
    /// TOCTOU ou deux evenements concurrents du meme user pouvaient tous deux
    /// passer le `check` avant le premier `set`. Ici le shard de la cle reste
    /// verrouille entre lecture et ecriture via l'API `entry` de DashMap.
    pub fn check_and_set(&self, user_id: UserId) -> Option<u64> {
        let cd = self.cooldown();
        let now = Instant::now();

        // Cleanup inline avant le `entry` (retain verrouille tous les shards,
        // l'appeler en tenant le lock d'une entry risquerait un deadlock).
        if self.map.len() > 500 {
            self.map.retain(|_, ts| ts.elapsed().as_secs() < cd);
        }

        use dashmap::mapref::entry::Entry;
        match self.map.entry(user_id) {
            Entry::Occupied(mut e) => {
                let elapsed = e.get().elapsed().as_secs();
                if elapsed < cd {
                    return Some(cd - elapsed);
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

    fn uid(id: u64) -> UserId {
        UserId::new(id)
    }

    #[test]
    fn test_no_cooldown_initially() {
        let tracker = CooldownTracker::new();
        assert!(tracker.check_and_set(uid(1)).is_none());
    }

    #[test]
    fn test_cooldown_after_set() {
        let tracker = CooldownTracker::new();
        // 1er appel : autorise + pose le timestamp.
        assert!(tracker.check_and_set(uid(1)).is_none());
        // 2e appel immediat : encore en cooldown.
        let remaining = tracker.check_and_set(uid(1));
        assert!(remaining.is_some());
        assert!(remaining.unwrap() <= DEFAULT_COOLDOWN_SECS);
    }

    #[test]
    fn test_different_users_independent() {
        let tracker = CooldownTracker::new();
        assert!(tracker.check_and_set(uid(1)).is_none());
        // Meme user : bloque. Autre user : autorise.
        assert!(tracker.check_and_set(uid(1)).is_some());
        assert!(tracker.check_and_set(uid(2)).is_none());
    }
}
