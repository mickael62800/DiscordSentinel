use dashmap::DashMap;

/// Tracker de parrainages.
pub struct SponsorshipTracker {
    /// (guild_id, filleul_id) → parrain_id
    sponsors: DashMap<(u64, u64), u64>,
    /// (guild_id, parrain_id) → nombre de filleuls actifs
    counts: DashMap<(u64, u64), u32>,
}

impl SponsorshipTracker {
    pub fn new() -> Self {
        Self {
            sponsors: DashMap::new(),
            counts: DashMap::new(),
        }
    }

    /// Enregistre un parrainage. Retourne Err si les limites sont atteintes.
    pub fn sponsor(
        &self,
        guild_id: u64,
        parrain_id: u64,
        filleul_id: u64,
        max_active: u32,
    ) -> Result<(), &'static str> {
        if parrain_id == filleul_id {
            return Err("Vous ne pouvez pas vous parrainer vous-meme.");
        }

        if self.sponsors.contains_key(&(guild_id, filleul_id)) {
            return Err("Ce membre a deja un parrain.");
        }

        let count = self
            .counts
            .get(&(guild_id, parrain_id))
            .map(|c| *c)
            .unwrap_or(0);
        if count >= max_active {
            return Err("Vous avez atteint le nombre maximum de filleuls actifs.");
        }

        self.sponsors.insert((guild_id, filleul_id), parrain_id);
        *self.counts.entry((guild_id, parrain_id)).or_insert(0) += 1;

        Ok(())
    }

    /// Recupere le parrain d'un filleul.
    #[allow(dead_code)]
    pub fn get_sponsor(&self, guild_id: u64, filleul_id: u64) -> Option<u64> {
        self.sponsors.get(&(guild_id, filleul_id)).map(|v| *v)
    }

    /// Verifie si un membre est parraine.
    #[allow(dead_code)]
    pub fn is_sponsored(&self, guild_id: u64, user_id: u64) -> bool {
        self.sponsors.contains_key(&(guild_id, user_id))
    }

    /// Nombre de filleuls actifs d'un parrain.
    #[allow(dead_code)]
    pub fn active_count(&self, guild_id: u64, parrain_id: u64) -> u32 {
        self.counts
            .get(&(guild_id, parrain_id))
            .map(|c| *c)
            .unwrap_or(0)
    }

    /// Retire un parrainage (utilise pour le rollback si l'API echoue).
    pub fn remove_sponsor(&self, guild_id: u64, parrain_id: u64, filleul_id: u64) {
        if self.sponsors.remove(&(guild_id, filleul_id)).is_some() {
            if let Some(mut entry) = self.counts.get_mut(&(guild_id, parrain_id)) {
                *entry = entry.saturating_sub(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sponsor_success() {
        let tracker = SponsorshipTracker::new();
        assert!(tracker.sponsor(1, 100, 200, 3).is_ok());
        assert_eq!(tracker.get_sponsor(1, 200), Some(100));
        assert!(tracker.is_sponsored(1, 200));
        assert_eq!(tracker.active_count(1, 100), 1);
    }

    #[test]
    fn sponsor_self_fails() {
        let tracker = SponsorshipTracker::new();
        assert_eq!(
            tracker.sponsor(1, 100, 100, 3).unwrap_err(),
            "Vous ne pouvez pas vous parrainer vous-meme."
        );
    }

    #[test]
    fn already_sponsored_fails() {
        let tracker = SponsorshipTracker::new();
        tracker.sponsor(1, 100, 200, 3).unwrap();
        assert_eq!(
            tracker.sponsor(1, 300, 200, 3).unwrap_err(),
            "Ce membre a deja un parrain."
        );
    }

    #[test]
    fn max_sponsorships_reached() {
        let tracker = SponsorshipTracker::new();
        tracker.sponsor(1, 100, 201, 2).unwrap();
        tracker.sponsor(1, 100, 202, 2).unwrap();
        assert_eq!(
            tracker.sponsor(1, 100, 203, 2).unwrap_err(),
            "Vous avez atteint le nombre maximum de filleuls actifs."
        );
    }

    #[test]
    fn different_guilds_independent() {
        let tracker = SponsorshipTracker::new();
        tracker.sponsor(1, 100, 200, 3).unwrap();
        // Meme filleul dans un autre guild → ok
        assert!(tracker.sponsor(2, 100, 200, 3).is_ok());
    }

    #[test]
    fn not_sponsored_returns_none() {
        let tracker = SponsorshipTracker::new();
        assert_eq!(tracker.get_sponsor(1, 999), None);
        assert!(!tracker.is_sponsored(1, 999));
    }

    #[test]
    fn active_count_zero_initially() {
        let tracker = SponsorshipTracker::new();
        assert_eq!(tracker.active_count(1, 100), 0);
    }

    #[test]
    fn multiple_filleuls() {
        let tracker = SponsorshipTracker::new();
        tracker.sponsor(1, 100, 201, 5).unwrap();
        tracker.sponsor(1, 100, 202, 5).unwrap();
        tracker.sponsor(1, 100, 203, 5).unwrap();
        assert_eq!(tracker.active_count(1, 100), 3);
    }
}
