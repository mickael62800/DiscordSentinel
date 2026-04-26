use super::*;

#[test]
fn db_str_round_trip_all_kinds() {
    for k in CurseKind::ALL {
        let s = k.as_db_str();
        assert_eq!(CurseKind::from_db_str(s), Some(k), "round-trip pour {s}");
    }
}

#[test]
fn from_db_str_unknown_returns_none() {
    assert_eq!(CurseKind::from_db_str("foo"), None);
    assert_eq!(CurseKind::from_db_str(""), None);
}

#[test]
fn all_kinds_have_distinct_emojis_and_labels() {
    let mut emojis: Vec<&str> = CurseKind::ALL.iter().map(|k| k.emoji()).collect();
    let mut labels: Vec<&str> = CurseKind::ALL.iter().map(|k| k.label()).collect();
    emojis.sort();
    emojis.dedup();
    labels.sort();
    labels.dedup();
    assert_eq!(emojis.len(), 6);
    assert_eq!(labels.len(), 6);
}

#[test]
fn pick_by_index_wraps_modulo() {
    assert_eq!(pick_curse_by_index(0), CurseKind::Chicken);
    assert_eq!(pick_curse_by_index(5), CurseKind::Heartbreak);
    assert_eq!(pick_curse_by_index(6), CurseKind::Chicken);
    assert_eq!(pick_curse_by_index(13), CurseKind::Banana);
}

#[test]
fn lift_cost_is_double_cast_cost() {
    assert_eq!(lift_cost(CurseKind::Banana), CURSE_COST_COINS * 2);
    assert_eq!(lift_cost(CurseKind::Chicken), 600);
}

#[test]
fn banana_does_nothing_without_curse() {
    assert_eq!(apply_banana_to_d20(15, false, 0.0), 15);
    assert_eq!(apply_banana_to_d20(20, false, 0.0), 20);
}

#[test]
fn banana_fails_when_proba_under_threshold() {
    assert_eq!(apply_banana_to_d20(20, true, 0.0), 1);
    assert_eq!(apply_banana_to_d20(20, true, 0.29), 1);
}

#[test]
fn banana_passes_when_proba_at_or_above_threshold() {
    assert_eq!(apply_banana_to_d20(20, true, 0.30), 20);
    assert_eq!(apply_banana_to_d20(20, true, 0.99), 20);
}

#[test]
fn leaky_wallet_no_curse_returns_amount_unchanged() {
    assert_eq!(apply_leaky_wallet(100, false), (100, 0));
}

#[test]
fn leaky_wallet_subtracts_fixed_fee() {
    assert_eq!(apply_leaky_wallet(100, true), (90, 10));
    assert_eq!(apply_leaky_wallet(50, true), (40, 10));
}

#[test]
fn leaky_wallet_eats_everything_when_amount_too_small() {
    assert_eq!(apply_leaky_wallet(10, true), (0, 10));
    assert_eq!(apply_leaky_wallet(5, true), (0, 5));
}

#[test]
fn leaky_wallet_ignores_zero_or_negative() {
    assert_eq!(apply_leaky_wallet(0, true), (0, 0));
    assert_eq!(apply_leaky_wallet(-50, true), (-50, 0));
}

#[test]
fn insomnia_multiplies_weight() {
    assert_eq!(apply_insomnia_to_taunt_weight(2.0, true), 3.0);
    assert_eq!(apply_insomnia_to_taunt_weight(2.0, false), 2.0);
}

#[test]
fn constants_match_spec() {
    assert_eq!(CURSE_COST_COINS, 300);
    assert_eq!(CURSE_DURATION_HOURS, 24);
    assert_eq!(CURSE_LIFT_MULTIPLIER, 2);
    assert_eq!(BANANA_FAIL_PROBABILITY, 0.30);
    assert_eq!(LEAKY_WALLET_FEE_COINS, 10);
    assert_eq!(SLOWNESS_DELAY_SECS, 10);
}

#[test]
fn display_format_includes_emoji_and_label() {
    let s = format!("{}", CurseKind::Banana);
    assert!(s.contains("🍌"));
    assert!(s.contains("banane"));
}
