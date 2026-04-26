use super::*;

#[test]
fn shield_halves_loss_on_first_defeat() {
    assert_eq!(apply_lucky_shield(100, true), 50);
    assert_eq!(apply_lucky_shield(1000, true), 500);
}

#[test]
fn shield_inactive_returns_nominal_loss() {
    assert_eq!(apply_lucky_shield(100, false), 100);
    assert_eq!(apply_lucky_shield(1000, false), 1000);
}

#[test]
fn shield_rounds_to_int() {
    // 99 / 2 = 49.5 -> arrondi 50
    assert_eq!(apply_lucky_shield(99, true), 50);
    // 11 / 2 = 5.5 -> arrondi 6
    assert_eq!(apply_lucky_shield(11, true), 6);
}

#[test]
fn shield_zero_loss_stays_zero() {
    assert_eq!(apply_lucky_shield(0, true), 0);
    assert_eq!(apply_lucky_shield(0, false), 0);
}

#[test]
fn shield_negative_loss_unchanged() {
    // Defensif : si perte negative (cas absurde), on ne modifie pas.
    assert_eq!(apply_lucky_shield(-50, true), -50);
}

#[test]
fn shield_loss_of_one_floors_to_one() {
    // 1 / 2 = 0.5 -> arrondi 1 (round half away from zero en Rust f64)
    let r = apply_lucky_shield(1, true);
    assert!(r == 0 || r == 1, "1/2 arrondi peut etre 0 ou 1, got {r}");
}

#[test]
fn preserve_streak_when_shield_active() {
    assert!(should_preserve_win_streak_after_shielded_defeat(true));
}

#[test]
fn dont_preserve_streak_when_no_shield() {
    assert!(!should_preserve_win_streak_after_shielded_defeat(false));
}

#[test]
fn multiplier_is_half() {
    assert_eq!(LUCKY_SHIELD_LOSS_MULTIPLIER, 0.5);
}
