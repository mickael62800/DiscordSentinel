use super::*;
use crate::domain::entities::coude::economy_config::CoudeEconomyConfig;

fn ecfg() -> CoudeEconomyConfig {
    CoudeEconomyConfig::default()
}

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
    assert_eq!(lift_cost(CurseKind::Banana, &ecfg()), CURSE_COST_COINS * 2);
    assert_eq!(lift_cost(CurseKind::Chicken, &ecfg()), 600);
    // Le cout de levee suit le cout de lancement specifique a la kind.
    assert_eq!(
        lift_cost(CurseKind::Empoisonner, &ecfg()),
        CurseKind::Empoisonner.cost_coins(&ecfg()) * CURSE_LIFT_MULTIPLIER
    );
    assert_eq!(lift_cost(CurseKind::FausseAssurance, &ecfg()), 500 * 2);
    assert_eq!(lift_cost(CurseKind::Pancarte, &ecfg()), 150 * 2);
}

#[test]
fn custom_config_changes_curse_cost_and_lift() {
    let cfg = CoudeEconomyConfig {
        curse_cost_coins: 1000,
        curse_lift_multiplier: 3,
        ..CoudeEconomyConfig::default()
    };
    // Malediction classique -> cout de base surchargé.
    assert_eq!(CurseKind::Banana.cost_coins(&cfg), 1000);
    assert_eq!(lift_cost(CurseKind::Banana, &cfg), 3000);
    // Sabotage nommé -> cout dedié inchangé, mais multiplicateur applique.
    assert_eq!(lift_cost(CurseKind::Pancarte, &cfg), 150 * 3);
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
    assert_eq!(apply_leaky_wallet(100, false, &ecfg()), (100, 0));
}

#[test]
fn leaky_wallet_subtracts_fixed_fee() {
    assert_eq!(apply_leaky_wallet(100, true, &ecfg()), (90, 10));
    assert_eq!(apply_leaky_wallet(50, true, &ecfg()), (40, 10));
}

#[test]
fn leaky_wallet_eats_everything_when_amount_too_small() {
    assert_eq!(apply_leaky_wallet(10, true, &ecfg()), (0, 10));
    assert_eq!(apply_leaky_wallet(5, true, &ecfg()), (0, 5));
}

#[test]
fn leaky_wallet_ignores_zero_or_negative() {
    assert_eq!(apply_leaky_wallet(0, true, &ecfg()), (0, 0));
    assert_eq!(apply_leaky_wallet(-50, true, &ecfg()), (-50, 0));
}

#[test]
fn leaky_wallet_custom_fee() {
    let cfg = CoudeEconomyConfig {
        leaky_wallet_fee_coins: 25,
        ..CoudeEconomyConfig::default()
    };
    assert_eq!(apply_leaky_wallet(100, true, &cfg), (75, 25));
    assert_eq!(apply_leaky_wallet(20, true, &cfg), (0, 20));
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

#[test]
fn graisser_has_specific_cost_and_duration() {
    assert_eq!(CurseKind::Graisser.cost_coins(&ecfg()), 200);
    assert_eq!(CurseKind::Graisser.duration_hours(), 24);
}

#[test]
fn pancarte_has_specific_cost_and_duration() {
    assert_eq!(CurseKind::Pancarte.cost_coins(&ecfg()), 150);
    assert_eq!(CurseKind::Pancarte.duration_hours(), 24 * 7);
}

#[test]
fn classic_curses_use_default_cost_and_duration() {
    for k in [
        CurseKind::Chicken,
        CurseKind::Banana,
        CurseKind::LeakyWallet,
        CurseKind::Slowness,
        CurseKind::Insomnia,
        CurseKind::Heartbreak,
    ] {
        assert_eq!(k.cost_coins(&ecfg()), 300);
        assert_eq!(k.duration_hours(), 24);
    }
}

#[test]
fn graisser_round_trips_db_string() {
    assert_eq!(CurseKind::Graisser.as_db_str(), "graisser");
    assert_eq!(
        CurseKind::from_db_str("graisser"),
        Some(CurseKind::Graisser)
    );
}

#[test]
fn graisser_excluded_from_random_pool() {
    // Tous les sabotages sont des effets explicites — pas de tirage
    // aleatoire par /maudire.
    for k in CurseKind::ALL {
        assert_ne!(k, CurseKind::Pancarte);
        assert_ne!(k, CurseKind::Graisser);
        assert_ne!(k, CurseKind::Empoisonner);
        assert_ne!(k, CurseKind::FausseAssurance);
    }
}

#[test]
fn fausse_assurance_has_specific_cost_uses_duration() {
    assert_eq!(CurseKind::FausseAssurance.cost_coins(&ecfg()), 500);
    assert_eq!(CurseKind::FausseAssurance.duration_hours(), 24 * 7);
    assert_eq!(CurseKind::FausseAssurance.initial_uses(), Some(1));
    assert_eq!(FAUSSE_ASSURANCE_FEE_COINS, 200);
}

#[test]
fn fausse_assurance_round_trips_db_string() {
    assert_eq!(CurseKind::FausseAssurance.as_db_str(), "fausse_assurance");
    assert_eq!(
        CurseKind::from_db_str("fausse_assurance"),
        Some(CurseKind::FausseAssurance)
    );
}

#[test]
fn empoisonner_has_specific_cost_uses_duration() {
    assert_eq!(CurseKind::Empoisonner.cost_coins(&ecfg()), 400);
    assert_eq!(CurseKind::Empoisonner.duration_hours(), 24 * 7);
    assert_eq!(CurseKind::Empoisonner.initial_uses(), Some(3));
}

#[test]
fn classic_curses_have_no_initial_uses() {
    for k in [
        CurseKind::Chicken,
        CurseKind::Banana,
        CurseKind::LeakyWallet,
        CurseKind::Slowness,
        CurseKind::Insomnia,
        CurseKind::Heartbreak,
        CurseKind::Pancarte,
        CurseKind::Graisser,
    ] {
        assert_eq!(k.initial_uses(), None, "{:?} ne doit pas avoir d uses", k);
    }
}

#[test]
fn poison_redirect_zero_when_inactive() {
    assert_eq!(poison_redirect_amount(1000, false), 0);
}

#[test]
fn poison_redirect_10_percent_when_active() {
    assert_eq!(poison_redirect_amount(1000, true), 100);
    assert_eq!(poison_redirect_amount(50, true), 5);
}

#[test]
fn poison_redirect_zero_for_non_positive_gain() {
    assert_eq!(poison_redirect_amount(0, true), 0);
    assert_eq!(poison_redirect_amount(-100, true), 0);
}

#[test]
fn poison_redirect_floor() {
    // 9 * 0.10 = 0.9 -> floor to 0
    assert_eq!(poison_redirect_amount(9, true), 0);
    // 10 * 0.10 = 1.0
    assert_eq!(poison_redirect_amount(10, true), 1);
}
