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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matchmaking_handicap() {
        assert_eq!(matchmaking_handicap(5, 5), (1.0, false));
        assert_eq!(matchmaking_handicap(5, 3), (1.0, false));
        assert_eq!(matchmaking_handicap(8, 5), (0.8, false));
        assert_eq!(matchmaking_handicap(10, 5), (0.8, false));
        assert_eq!(matchmaking_handicap(12, 5), (0.6, false));
        assert_eq!(matchmaking_handicap(20, 5), (0.0, true));
    }
}
