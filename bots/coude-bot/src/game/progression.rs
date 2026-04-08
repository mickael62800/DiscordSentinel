/// Niveau maximum.
pub const MAX_LEVEL: i32 = 25;

/// XP necessaire pour passer du niveau `level` au suivant.
pub fn xp_for_level(level: i32) -> i64 {
    (50 * level * level + 50 * level) as i64
}

/// Titre correspondant au niveau.
pub fn title_for_level(level: i32) -> &'static str {
    match level {
        1..=4 => "Debutant",
        5..=9 => "Bagarreur",
        10..=14 => "Guerrier",
        15..=19 => "Veteran",
        20..=24 => "Champion",
        25 => "Inarretable",
        _ => "Debutant",
    }
}

/// Verifie si le joueur devrait monter de niveau.
/// Retourne le nouveau niveau si level up, None sinon.
#[allow(dead_code)]
pub fn check_level_up(current_level: i32, current_xp: i64) -> Option<i32> {
    if current_level >= MAX_LEVEL {
        return None;
    }
    let needed = xp_for_level(current_level);
    if current_xp >= needed {
        Some(current_level + 1)
    } else {
        None
    }
}

/// Calcule le handicap de matchmaking.
/// Retourne (multiplicateur_atk_pour_le_plus_fort, est_bloque).
pub fn matchmaking_handicap(attacker_level: i32, defender_level: i32) -> (f64, bool) {
    let gap = (attacker_level - defender_level).abs();
    match gap {
        0..=2 => (1.0, false),
        3..=5 => (0.8, false),
        6..=9 => (0.6, false),
        _ => (0.0, true),
    }
}

/// Calcule les HP max d'un joueur : 100 + DEF_effective * 2.
/// Remplace l'ancien display_hp (100 + DEF/2) qui etait cosmetique.
pub fn calculate_hp_max(effective_def: i32) -> i32 {
    100 + effective_def * 2
}

/// Alias pour compatibilite.
#[allow(dead_code)]
pub fn display_hp(effective_def: i32) -> i32 {
    calculate_hp_max(effective_def)
}

/// Calcule la regeneration naturelle de HP depuis la derniere mise a jour.
/// +10 HP par heure, plafonne a hp_max.
#[allow(dead_code)]
pub fn calculate_regen(hp_current: i32, hp_max: i32, hours_elapsed: f64) -> i32 {
    let regen = (hours_elapsed * 10.0) as i32;
    (hp_current + regen).min(hp_max)
}

/// Nombre de rounds max selon les HP combines des deux joueurs.
#[allow(dead_code)]
pub fn max_rounds(combined_hp: i32) -> i32 {
    if combined_hp < 250 {
        3
    } else if combined_hp <= 400 {
        5
    } else {
        7
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xp_for_level() {
        assert_eq!(xp_for_level(1), 100);
        assert_eq!(xp_for_level(2), 300);
        assert_eq!(xp_for_level(5), 1500);
        assert_eq!(xp_for_level(10), 5500);
    }

    #[test]
    fn test_title_for_level() {
        assert_eq!(title_for_level(1), "Debutant");
        assert_eq!(title_for_level(4), "Debutant");
        assert_eq!(title_for_level(5), "Bagarreur");
        assert_eq!(title_for_level(10), "Guerrier");
        assert_eq!(title_for_level(15), "Veteran");
        assert_eq!(title_for_level(20), "Champion");
        assert_eq!(title_for_level(25), "Inarretable");
    }

    #[test]
    fn test_check_level_up() {
        // Level 1 needs 100 XP
        assert_eq!(check_level_up(1, 99), None);
        assert_eq!(check_level_up(1, 100), Some(2));
        assert_eq!(check_level_up(1, 200), Some(2));
        // Max level cannot level up
        assert_eq!(check_level_up(25, 999999), None);
    }

    #[test]
    fn test_matchmaking_handicap() {
        assert_eq!(matchmaking_handicap(5, 5), (1.0, false));
        assert_eq!(matchmaking_handicap(5, 3), (1.0, false));
        assert_eq!(matchmaking_handicap(8, 5), (0.8, false));
        assert_eq!(matchmaking_handicap(10, 5), (0.8, false));
        assert_eq!(matchmaking_handicap(12, 5), (0.6, false));
        assert_eq!(matchmaking_handicap(20, 5), (0.0, true));
    }

    #[test]
    fn test_display_hp() {
        assert_eq!(display_hp(8), 104);
        assert_eq!(display_hp(25), 112);
        assert_eq!(display_hp(100), 150);
    }
}
