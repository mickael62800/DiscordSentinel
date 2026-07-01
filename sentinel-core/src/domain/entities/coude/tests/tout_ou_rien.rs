use super::*;
use crate::domain::entities::coude::economy_config::CoudeEconomyConfig;

fn cfg() -> CoudeEconomyConfig {
    CoudeEconomyConfig::default()
}

#[test]
fn win_doubles_balance() {
    assert_eq!(coin_delta(1000, ToutOuRienOutcome::Win, &cfg()), 1000);
    assert_eq!(coin_delta(50_000, ToutOuRienOutcome::Win, &cfg()), 50_000);
}

#[test]
fn lose_takes_80_percent() {
    assert_eq!(coin_delta(1000, ToutOuRienOutcome::Lose, &cfg()), -800);
    assert_eq!(coin_delta(50_000, ToutOuRienOutcome::Lose, &cfg()), -40_000);
}

#[test]
fn zero_balance_no_change() {
    assert_eq!(coin_delta(0, ToutOuRienOutcome::Win, &cfg()), 0);
    assert_eq!(coin_delta(0, ToutOuRienOutcome::Lose, &cfg()), 0);
}

#[test]
fn negative_balance_no_change() {
    // Si le wallet est deja en negatif (impossible en pratique mais
    // garde-fou), pas de mutation supplementaire.
    assert_eq!(coin_delta(-100, ToutOuRienOutcome::Win, &cfg()), 0);
    assert_eq!(coin_delta(-100, ToutOuRienOutcome::Lose, &cfg()), 0);
}

#[test]
fn resolve_outcome_below_threshold_wins() {
    assert_eq!(resolve_outcome(0.0, &cfg()), ToutOuRienOutcome::Win);
    assert_eq!(resolve_outcome(0.49, &cfg()), ToutOuRienOutcome::Win);
}

#[test]
fn resolve_outcome_at_or_above_threshold_loses() {
    assert_eq!(resolve_outcome(0.5, &cfg()), ToutOuRienOutcome::Lose);
    assert_eq!(resolve_outcome(0.99, &cfg()), ToutOuRienOutcome::Lose);
}

#[test]
fn constants_match_spec() {
    assert_eq!(TOUT_OU_RIEN_WIN_PROBABILITY, 0.5);
    assert_eq!(TOUT_OU_RIEN_WIN_MULTIPLIER, 2.0);
    assert_eq!(TOUT_OU_RIEN_LOSS_KEEP_PCT, 0.20);
    assert_eq!(TOUT_OU_RIEN_COOLDOWN_SECS, 604_800);
    assert_eq!(TOUT_OU_RIEN_COOLDOWN_KEY, "tout_ou_rien");
}

#[test]
fn small_balance_lose_floor_is_correct() {
    // 5c -> -(5 * 0.8) = -4 (le joueur garde 1c).
    assert_eq!(coin_delta(5, ToutOuRienOutcome::Lose, &cfg()), -4);
    assert_eq!(coin_delta(1, ToutOuRienOutcome::Lose, &cfg()), 0);
}

#[test]
fn custom_config_changes_multiplier_and_keep() {
    // Multiplicateur x3 -> gain = +2*balance ; keep 50% -> perte = -50%.
    let c = CoudeEconomyConfig {
        tout_ou_rien_win_multiplier: 3.0,
        tout_ou_rien_loss_keep_pct: 0.5,
        tout_ou_rien_win_probability: 0.7,
        ..CoudeEconomyConfig::default()
    };
    assert_eq!(coin_delta(1000, ToutOuRienOutcome::Win, &c), 2000);
    assert_eq!(coin_delta(1000, ToutOuRienOutcome::Lose, &c), -500);
    assert_eq!(resolve_outcome(0.65, &c), ToutOuRienOutcome::Win);
    assert_eq!(resolve_outcome(0.75, &c), ToutOuRienOutcome::Lose);
}
