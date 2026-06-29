use dashmap::DashMap;

/// Donnees de streak pour un utilisateur.
#[derive(Debug, Clone)]
pub struct StreakData {
    pub last_active_day: u32,
    pub last_active_year: i32,
    pub current_streak: u32,
    pub best_streak: u32,
}

/// Resultat d'une mise a jour de streak.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StreakUpdate {
    /// True si c'est un nouveau jour d'activite.
    pub new_day: bool,
    /// Streak actuel apres mise a jour.
    pub current_streak: u32,
    /// Multiplicateur XP bonus (1.0 = pas de bonus).
    pub xp_multiplier: f64,
}

/// Tracker des streaks par (guild, user).
pub struct StreakTracker {
    streaks: DashMap<(u64, u64), StreakData>,
}

impl StreakTracker {
    /// Precharge un streak depuis les donnees API (au premier message apres restart).
    pub fn seed(
        &self,
        guild_id: u64,
        user_id: u64,
        current: u32,
        best: u32,
        last_day: u32,
        last_year: i32,
    ) {
        // Ne pas ecraser si deja present (activite en cours)
        self.streaks
            .entry((guild_id, user_id))
            .or_insert(StreakData {
                last_active_day: last_day,
                last_active_year: last_year,
                current_streak: current,
                best_streak: best,
            });
    }

    pub fn new() -> Self {
        Self {
            streaks: DashMap::new(),
        }
    }

    /// Enregistre l'activite d'un utilisateur.
    /// Retourne les infos de streak mises a jour.
    pub fn record_activity(
        &self,
        guild_id: u64,
        user_id: u64,
        day_of_year: u32,
        year: i32,
    ) -> StreakUpdate {
        let mut entry = self
            .streaks
            .entry((guild_id, user_id))
            .or_insert(StreakData {
                last_active_day: 0,
                last_active_year: 0,
                current_streak: 0,
                best_streak: 0,
            });

        let data = entry.value_mut();

        // Meme jour → pas de mise a jour
        if data.last_active_day == day_of_year && data.last_active_year == year {
            return StreakUpdate {
                new_day: false,
                current_streak: data.current_streak,
                xp_multiplier: streak_multiplier(data.current_streak),
            };
        }

        // Jour suivant (ou premier jour de la nouvelle annee) → streak continue
        let is_consecutive = is_next_day(
            data.last_active_day,
            data.last_active_year,
            day_of_year,
            year,
        );

        if is_consecutive {
            data.current_streak += 1;
        } else {
            data.current_streak = 1;
        }

        if data.current_streak > data.best_streak {
            data.best_streak = data.current_streak;
        }

        data.last_active_day = day_of_year;
        data.last_active_year = year;

        StreakUpdate {
            new_day: true,
            current_streak: data.current_streak,
            xp_multiplier: streak_multiplier(data.current_streak),
        }
    }

    /// Recupere le streak actuel d'un utilisateur.
    /// Verifie si un utilisateur est dans le cache de streaks.
    pub fn has(&self, guild_id: u64, user_id: u64) -> bool {
        self.streaks.contains_key(&(guild_id, user_id))
    }

    pub fn get_streak(&self, guild_id: u64, user_id: u64) -> (u32, u32) {
        self.streaks
            .get(&(guild_id, user_id))
            .map(|d| (d.current_streak, d.best_streak))
            .unwrap_or((0, 0))
    }
}

/// Calcule le multiplicateur XP bonus en fonction du streak.
/// 1.0 base + 0.1 par semaine complete, max 1.5x (a 35 jours).
pub fn streak_multiplier(streak_days: u32) -> f64 {
    let bonus = (streak_days / 7) as f64 * 0.1;
    (1.0 + bonus).min(1.5)
}

/// Verifie si (day2, year2) est le jour suivant (day1, year1).
fn is_next_day(day1: u32, year1: i32, day2: u32, year2: i32) -> bool {
    if year1 == year2 {
        day2 == day1 + 1
    } else if year2 == year1 + 1 {
        // Passage d'annee : day1 = 365 ou 366, day2 = 1
        day1 >= 365 && day2 == 1
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_activity() {
        let tracker = StreakTracker::new();
        let update = tracker.record_activity(1, 100, 42, 2025);
        assert!(update.new_day);
        assert_eq!(update.current_streak, 1);
        assert_eq!(update.xp_multiplier, 1.0);
    }

    #[test]
    fn same_day_no_update() {
        let tracker = StreakTracker::new();
        tracker.record_activity(1, 100, 42, 2025);
        let update = tracker.record_activity(1, 100, 42, 2025);
        assert!(!update.new_day);
        assert_eq!(update.current_streak, 1);
    }

    #[test]
    fn consecutive_days() {
        let tracker = StreakTracker::new();
        tracker.record_activity(1, 100, 42, 2025);
        let update = tracker.record_activity(1, 100, 43, 2025);
        assert!(update.new_day);
        assert_eq!(update.current_streak, 2);
    }

    #[test]
    fn streak_broken() {
        let tracker = StreakTracker::new();
        tracker.record_activity(1, 100, 42, 2025);
        tracker.record_activity(1, 100, 43, 2025);
        // Skip day 44
        let update = tracker.record_activity(1, 100, 45, 2025);
        assert_eq!(update.current_streak, 1); // Reset
    }

    #[test]
    fn streak_across_year() {
        let tracker = StreakTracker::new();
        tracker.record_activity(1, 100, 365, 2025);
        let update = tracker.record_activity(1, 100, 1, 2026);
        assert_eq!(update.current_streak, 2); // Continue
    }

    #[test]
    fn best_streak_tracked() {
        let tracker = StreakTracker::new();
        // 3 jours consecutifs
        tracker.record_activity(1, 100, 10, 2025);
        tracker.record_activity(1, 100, 11, 2025);
        tracker.record_activity(1, 100, 12, 2025);
        // Break
        tracker.record_activity(1, 100, 20, 2025);
        let (current, best) = tracker.get_streak(1, 100);
        assert_eq!(current, 1);
        assert_eq!(best, 3);
    }

    #[test]
    fn multiplier_base() {
        assert_eq!(streak_multiplier(0), 1.0);
        assert_eq!(streak_multiplier(1), 1.0);
        assert_eq!(streak_multiplier(6), 1.0);
    }

    #[test]
    fn multiplier_week() {
        assert_eq!(streak_multiplier(7), 1.1);
        assert_eq!(streak_multiplier(14), 1.2);
    }

    #[test]
    fn multiplier_capped() {
        assert_eq!(streak_multiplier(35), 1.5);
        assert_eq!(streak_multiplier(100), 1.5); // Capped
    }

    #[test]
    fn different_users_independent() {
        let tracker = StreakTracker::new();
        tracker.record_activity(1, 100, 42, 2025);
        tracker.record_activity(1, 100, 43, 2025);
        tracker.record_activity(1, 200, 43, 2025);
        let (s1, _) = tracker.get_streak(1, 100);
        let (s2, _) = tracker.get_streak(1, 200);
        assert_eq!(s1, 2);
        assert_eq!(s2, 1);
    }

    // ── Tests pour seed() et has() (reload depuis API) ──

    #[test]
    fn seed_loads_existing_streak() {
        let tracker = StreakTracker::new();
        tracker.seed(1, 100, 15, 20, 100, 2025);
        assert!(tracker.has(1, 100));
        let (current, best) = tracker.get_streak(1, 100);
        assert_eq!(current, 15);
        assert_eq!(best, 20);
    }

    #[test]
    fn seed_does_not_overwrite_active() {
        let tracker = StreakTracker::new();
        tracker.record_activity(1, 100, 42, 2025);
        // Seed avec des valeurs differentes — ne doit PAS ecraser
        tracker.seed(1, 100, 99, 99, 1, 2020);
        let (current, _) = tracker.get_streak(1, 100);
        assert_eq!(current, 1); // valeur du record_activity, pas du seed
    }

    #[test]
    fn has_returns_false_for_unknown() {
        let tracker = StreakTracker::new();
        assert!(!tracker.has(1, 999));
    }

    #[test]
    fn seed_then_record_continues_streak() {
        let tracker = StreakTracker::new();
        // Simuler un reload : dernier jour actif = 100, streak = 5
        tracker.seed(1, 100, 5, 10, 100, 2025);
        // Le jour suivant (101)
        let update = tracker.record_activity(1, 100, 101, 2025);
        assert!(update.new_day);
        assert_eq!(update.current_streak, 6);
    }

    #[test]
    fn seed_then_record_breaks_streak_if_gap() {
        let tracker = StreakTracker::new();
        tracker.seed(1, 100, 5, 10, 100, 2025);
        // Jour 103 — gap de 2 jours
        let update = tracker.record_activity(1, 100, 103, 2025);
        assert!(update.new_day);
        assert_eq!(update.current_streak, 1); // streak cassee
    }
}
