//! Garde de deduplication des suggestions anti-raid.
//!
//! En mode `suggest`/`hybrid`, une vague de joins limite (borderline) ne doit
//! pas spammer le salon staff avec une alerte par membre. Ce garde memorise,
//! par serveur, l'instant de la derniere suggestion postee et refuse d'en
//! reposter une tant que le TTL n'est pas ecoule (ou qu'une confirmation ne
//! l'a pas explicitement liberee).

use std::time::{Duration, Instant};

use dashmap::DashMap;
use serenity::model::id::GuildId;

/// TTL par defaut d'une suggestion en attente (anti-spam).
pub const SUGGEST_TTL_SECS: u64 = 300;

/// Garde en memoire : guild_id -> instant de la derniere suggestion postee.
pub struct RaidSuggestGuard {
    pending: DashMap<GuildId, Instant>,
    ttl: Duration,
}

impl RaidSuggestGuard {
    pub fn new() -> Self {
        Self::with_ttl(SUGGEST_TTL_SECS)
    }

    pub fn with_ttl(ttl_secs: u64) -> Self {
        Self {
            pending: DashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Tente d'acquerir le droit de poster une suggestion pour ce serveur.
    /// Retourne `true` si aucune suggestion recente n'est en attente (et
    /// enregistre alors l'instant courant), `false` sinon (dedupe).
    pub fn try_acquire(&self, guild_id: GuildId) -> bool {
        let now = Instant::now();
        if let Some(entry) = self.pending.get(&guild_id) {
            if now.duration_since(*entry.value()) < self.ttl {
                return false;
            }
        }
        self.pending.insert(guild_id, now);
        true
    }

    /// Libere le garde (suggestion confirmee ou ignoree) pour autoriser une
    /// nouvelle alerte immediatement.
    pub fn release(&self, guild_id: GuildId) {
        self.pending.remove(&guild_id);
    }
}

impl Default for RaidSuggestGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_acquire_succeeds_then_dedupes() {
        let guard = RaidSuggestGuard::new();
        let g = GuildId::new(1);
        assert!(guard.try_acquire(g));
        // Immediatement apres : dedupe.
        assert!(!guard.try_acquire(g));
    }

    #[test]
    fn release_allows_new_suggestion() {
        let guard = RaidSuggestGuard::new();
        let g = GuildId::new(1);
        assert!(guard.try_acquire(g));
        guard.release(g);
        assert!(guard.try_acquire(g));
    }

    #[test]
    fn expired_ttl_allows_new_suggestion() {
        let guard = RaidSuggestGuard::with_ttl(0);
        let g = GuildId::new(1);
        assert!(guard.try_acquire(g));
        // TTL de 0 => l'entree est consideree expiree immediatement.
        assert!(guard.try_acquire(g));
    }

    #[test]
    fn distinct_guilds_are_independent() {
        let guard = RaidSuggestGuard::new();
        assert!(guard.try_acquire(GuildId::new(1)));
        assert!(guard.try_acquire(GuildId::new(2)));
    }
}
