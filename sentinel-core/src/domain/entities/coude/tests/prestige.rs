use super::*;

#[test]
fn cannot_prestige_below_level_25() {
    assert!(!can_prestige(1, 0));
    assert!(!can_prestige(24, 0));
}

#[test]
fn can_prestige_at_level_25() {
    assert!(can_prestige(25, 0));
    assert!(can_prestige(50, 2));
}

#[test]
fn cannot_prestige_at_max_count() {
    assert!(!can_prestige(50, PRESTIGE_MAX_COUNT));
    assert!(!can_prestige(50, 10));
}

#[test]
fn gain_multiplier_neutral_at_zero() {
    assert_eq!(prestige_gain_multiplier(0), 1.0);
}

#[test]
fn gain_multiplier_steps_5_percent() {
    assert!((prestige_gain_multiplier(1) - 1.05).abs() < 0.001);
    assert!((prestige_gain_multiplier(3) - 1.15).abs() < 0.001);
    assert!((prestige_gain_multiplier(5) - 1.25).abs() < 0.001);
}

#[test]
fn gain_multiplier_clamps_at_max() {
    let cap = prestige_gain_multiplier(5);
    assert_eq!(prestige_gain_multiplier(10), cap);
    assert_eq!(prestige_gain_multiplier(100), cap);
}

#[test]
fn stars_render_correctly() {
    assert_eq!(prestige_stars(0), "");
    assert_eq!(prestige_stars(1), "\u{2b50}");
    assert_eq!(prestige_stars(3), "\u{2b50}\u{2b50}\u{2b50}");
    assert_eq!(prestige_stars(5), "\u{2b50}".repeat(5));
}

#[test]
fn stars_clamp_at_max() {
    assert_eq!(prestige_stars(10), prestige_stars(PRESTIGE_MAX_COUNT));
}

#[test]
fn gain_multiplier_negative_max_count_does_not_panic() {
    // Une config corrompue avec max_count negatif ne doit pas paniquer
    // (i32::clamp panique si min > max). On retombe sur le neutre.
    assert_eq!(
        prestige_gain_multiplier_with_params(3, PRESTIGE_GAIN_BONUS_PCT, -1),
        1.0
    );
    assert_eq!(
        prestige_gain_multiplier_with_params(0, PRESTIGE_GAIN_BONUS_PCT, -10),
        1.0
    );
}

#[test]
fn constants_match_spec() {
    assert_eq!(PRESTIGE_UNLOCK_LEVEL, 25);
    assert_eq!(PRESTIGE_MAX_COUNT, 5);
    assert_eq!(PRESTIGE_GAIN_BONUS_PCT, 0.05);
}
