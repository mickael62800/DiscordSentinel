use super::*;

#[test]
fn win_doubles_balance() {
    assert_eq!(coin_delta(1000, ToutOuRienOutcome::Win), 1000);
    assert_eq!(coin_delta(50_000, ToutOuRienOutcome::Win), 50_000);
}

#[test]
fn lose_takes_80_percent() {
    assert_eq!(coin_delta(1000, ToutOuRienOutcome::Lose), -800);
    assert_eq!(coin_delta(50_000, ToutOuRienOutcome::Lose), -40_000);
}

#[test]
fn zero_balance_no_change() {
    assert_eq!(coin_delta(0, ToutOuRienOutcome::Win), 0);
    assert_eq!(coin_delta(0, ToutOuRienOutcome::Lose), 0);
}

#[test]
fn negative_balance_no_change() {
    // Si le wallet est deja en negatif (impossible en pratique mais
    // garde-fou), pas de mutation supplementaire.
    assert_eq!(coin_delta(-100, ToutOuRienOutcome::Win), 0);
    assert_eq!(coin_delta(-100, ToutOuRienOutcome::Lose), 0);
}

#[test]
fn resolve_outcome_below_threshold_wins() {
    assert_eq!(resolve_outcome(0.0), ToutOuRienOutcome::Win);
    assert_eq!(resolve_outcome(0.49), ToutOuRienOutcome::Win);
}

#[test]
fn resolve_outcome_at_or_above_threshold_loses() {
    assert_eq!(resolve_outcome(0.5), ToutOuRienOutcome::Lose);
    assert_eq!(resolve_outcome(0.99), ToutOuRienOutcome::Lose);
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
    assert_eq!(coin_delta(5, ToutOuRienOutcome::Lose), -4);
    assert_eq!(coin_delta(1, ToutOuRienOutcome::Lose), 0);
}
