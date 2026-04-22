use super::*;

#[test]
fn default_combats_limit_is_50() {
    assert_eq!(DEFAULT_COUDE_COMBATS_LIMIT, 50);
}

#[test]
fn default_opponent_count_is_2_duel() {
    // Regle metier : 2 = duel 1v1, le mode standard Coup de Coude.
    assert_eq!(DEFAULT_COUDE_OPPONENT_COUNT, 2);
}

#[test]
fn default_social_leaderboard_is_10() {
    assert_eq!(DEFAULT_COUDE_SOCIAL_LEADERBOARD_LIMIT, 10);
}
