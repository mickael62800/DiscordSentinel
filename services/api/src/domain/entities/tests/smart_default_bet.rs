use super::*;

// ── suggest_default_bet ──

#[test]
fn suggests_20_pct_within_bounds() {
    // 1000 wallet, 20% = 200, dans [10, 1000] -> 200
    assert_eq!(suggest_default_bet(1000, 10, 1000), 200);
}

#[test]
fn suggestion_clamped_to_min_when_wallet_small() {
    // 30 wallet, 20% = 6, mais min = 10 -> 10
    assert_eq!(suggest_default_bet(30, 10, 1000), 10);
}

#[test]
fn suggestion_clamped_to_max_when_wallet_huge() {
    // 100000 wallet, 20% = 20000, mais max = 1000 -> 1000
    assert_eq!(suggest_default_bet(100_000, 10, 1000), 1000);
}

#[test]
fn zero_wallet_returns_min_bet() {
    assert_eq!(suggest_default_bet(0, 10, 1000), 10);
}

#[test]
fn negative_wallet_treated_as_zero() {
    // Defensif : wallet ne devrait jamais etre negatif mais on veut pas
    // crash ici si ca arrive (overdraft, bug).
    assert_eq!(suggest_default_bet(-100, 10, 1000), 10);
}

#[test]
fn rounding_to_int() {
    // 33 wallet, 20% = 6.6 -> arrondi 7, mais < min 10 -> 10
    assert_eq!(suggest_default_bet(33, 10, 1000), 10);
    // 53 wallet, 20% = 10.6 -> arrondi 11, dans [10, 1000] -> 11
    assert_eq!(suggest_default_bet(53, 10, 1000), 11);
}

#[test]
fn invalid_config_max_below_min_falls_back_to_min() {
    // Config absurde max < min -> retourne min
    assert_eq!(suggest_default_bet(1000, 100, 50), 100);
}

#[test]
fn pct_constant_is_20_percent() {
    assert_eq!(DEFAULT_BET_PCT, 0.20);
}

// ── quick_bet_buttons ──

#[test]
fn quick_buttons_basic_3_multipliers() {
    let r = quick_bet_buttons(50, 1000, 10, 1000, &[1, 2, 5], false);
    // 50, 100, 250
    assert_eq!(r, vec![50, 100, 250]);
}

#[test]
fn quick_buttons_with_all_in() {
    let r = quick_bet_buttons(50, 800, 10, 1000, &[1, 2, 5], true);
    // 50, 100, 250, 800 (all-in)
    assert_eq!(r, vec![50, 100, 250, 800]);
}

#[test]
fn quick_buttons_clamp_to_max() {
    let r = quick_bet_buttons(300, 5000, 10, 1000, &[1, 2, 5], false);
    // 300, 600, 1500 -> 300, 600, 1000 (clampe), dedup
    assert_eq!(r, vec![300, 600, 1000]);
}

#[test]
fn quick_buttons_dedup_after_clamp() {
    let r = quick_bet_buttons(1000, 5000, 10, 1000, &[1, 2, 5], false);
    // 1000, 2000, 5000 tous clampes a 1000 -> dedup -> [1000]
    assert_eq!(r, vec![1000]);
}

#[test]
fn quick_buttons_all_in_clamped_to_max() {
    let r = quick_bet_buttons(50, 99999, 10, 1000, &[1], true);
    // 50, all-in clampe a 1000
    assert_eq!(r, vec![50, 1000]);
}

#[test]
fn quick_buttons_all_in_floored_to_min_when_poor() {
    let r = quick_bet_buttons(10, 5, 10, 1000, &[1], true);
    // 10, all-in clampe a 10 -> dedup -> [10]
    assert_eq!(r, vec![10]);
}

#[test]
fn quick_buttons_empty_multipliers() {
    let r = quick_bet_buttons(50, 1000, 10, 1000, &[], true);
    // Juste all-in = 1000
    assert_eq!(r, vec![1000]);
}

#[test]
fn quick_buttons_no_all_in_no_multipliers_returns_empty() {
    let r = quick_bet_buttons(50, 1000, 10, 1000, &[], false);
    assert!(r.is_empty());
}
