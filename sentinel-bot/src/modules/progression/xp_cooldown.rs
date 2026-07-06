use std::time::{Duration, Instant};

use dashmap::DashMap;

/// Cooldown anti-farm pour l'XP par message.
pub struct XpCooldown {
    last_xp: DashMap<(u64, u64), Instant>,
}

impl XpCooldown {
    pub fn new() -> Self {
        Self {
            last_xp: DashMap::new(),
        }
    }

    /// Verifie si l'utilisateur peut gagner de l'XP (cooldown expire). Conserve
    /// pour les tests ; le chemin de prod utilise `try_claim` (atomique).
    #[allow(dead_code)]
    pub fn can_gain_xp(&self, guild_id: u64, user_id: u64, cooldown_secs: u64) -> bool {
        if cooldown_secs == 0 {
            return true;
        }

        let cooldown = Duration::from_secs(cooldown_secs);
        let now = Instant::now();

        match self.last_xp.get(&(guild_id, user_id)) {
            Some(last) => now.duration_since(*last) >= cooldown,
            None => true,
        }
    }

    /// Reserve ATOMIQUEMENT le gain d'XP : renvoie `true` (et pose le timestamp)
    /// si le cooldown est expire, `false` sinon. Remplace le couple non atomique
    /// can_gain_xp + record_xp (deux messages concurrents pouvaient tous deux
    /// passer le check avant le record -> double credit). Le lock d'entree
    /// DashMap rend le compare-and-set indivisible.
    pub fn try_claim(&self, guild_id: u64, user_id: u64, cooldown_secs: u64) -> bool {
        let now = Instant::now();
        if cooldown_secs == 0 {
            self.last_xp.insert((guild_id, user_id), now);
            return true;
        }
        let cooldown = Duration::from_secs(cooldown_secs);
        let mut allowed = false;
        self.last_xp
            .entry((guild_id, user_id))
            .and_modify(|last| {
                if now.duration_since(*last) >= cooldown {
                    *last = now;
                    allowed = true;
                }
            })
            .or_insert_with(|| {
                allowed = true;
                now
            });
        if self.last_xp.len() > 10_000 {
            self.cleanup(300);
        }
        allowed
    }

    /// Enregistre le gain d'XP pour le cooldown.
    #[allow(dead_code)]
    pub fn record_xp(&self, guild_id: u64, user_id: u64) {
        self.last_xp.insert((guild_id, user_id), Instant::now());

        // Nettoyage periodique : toutes les 10 000 insertions environ
        if self.last_xp.len() > 10_000 {
            self.cleanup(300); // Supprimer les entrees > 5 min
        }
    }

    /// Supprime les entrees plus vieilles que `max_age_secs`.
    pub fn cleanup(&self, max_age_secs: u64) {
        let cutoff = Duration::from_secs(max_age_secs);
        let now = Instant::now();
        self.last_xp
            .retain(|_, last| now.duration_since(*last) < cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_message_allowed() {
        let cooldown = XpCooldown::new();
        assert!(cooldown.can_gain_xp(1, 100, 60));
    }

    #[test]
    fn second_message_blocked() {
        let cooldown = XpCooldown::new();
        cooldown.record_xp(1, 100);
        assert!(!cooldown.can_gain_xp(1, 100, 60));
    }

    #[test]
    fn cooldown_zero_always_allowed() {
        let cooldown = XpCooldown::new();
        cooldown.record_xp(1, 100);
        assert!(cooldown.can_gain_xp(1, 100, 0));
    }

    #[test]
    fn different_users_independent() {
        let cooldown = XpCooldown::new();
        cooldown.record_xp(1, 100);
        assert!(cooldown.can_gain_xp(1, 200, 60)); // User 200 not on cooldown
    }

    #[test]
    fn different_guilds_independent() {
        let cooldown = XpCooldown::new();
        cooldown.record_xp(1, 100);
        assert!(cooldown.can_gain_xp(2, 100, 60)); // Guild 2 not on cooldown
    }
}
