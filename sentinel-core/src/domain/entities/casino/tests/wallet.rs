use super::*;

#[test]
fn clamp_debit_within_balance() {
    assert_eq!(clamp_debit_to_balance(50, 100), 50);
}

#[test]
fn clamp_debit_equal_to_balance() {
    assert_eq!(clamp_debit_to_balance(100, 100), 100);
}

#[test]
fn clamp_debit_exceeds_balance_returns_balance() {
    assert_eq!(clamp_debit_to_balance(500, 100), 100);
}

#[test]
fn clamp_debit_negative_amount_returns_zero() {
    assert_eq!(clamp_debit_to_balance(-50, 100), 0);
}

#[test]
fn clamp_debit_negative_balance_returns_zero() {
    // Balance negative (corruption DB ?) → pas de debit.
    assert_eq!(clamp_debit_to_balance(100, -50), 0);
}

#[test]
fn clamp_debit_zero_amount() {
    assert_eq!(clamp_debit_to_balance(0, 100), 0);
}

#[test]
fn clamp_debit_zero_balance() {
    assert_eq!(clamp_debit_to_balance(100, 0), 0);
}

#[test]
fn clamp_debit_both_negative() {
    assert_eq!(clamp_debit_to_balance(-1, -1), 0);
}

#[test]
fn clamp_debit_large_values_no_overflow() {
    assert_eq!(clamp_debit_to_balance(i64::MAX, 1000), 1000);
    assert_eq!(clamp_debit_to_balance(1000, i64::MAX), 1000);
}

// ── resolve_starting_coins ──

#[test]
fn starting_coins_default_when_env_absent() {
    assert_eq!(resolve_starting_coins(None), DEFAULT_STARTING_COINS);
    assert_eq!(resolve_starting_coins(None), 100);
}

#[test]
fn starting_coins_uses_env_when_valid() {
    assert_eq!(resolve_starting_coins(Some("500")), 500);
    assert_eq!(resolve_starting_coins(Some("0")), 0);
}

#[test]
fn starting_coins_fallback_on_invalid_env() {
    assert_eq!(resolve_starting_coins(Some("abc")), DEFAULT_STARTING_COINS);
    assert_eq!(resolve_starting_coins(Some("")), DEFAULT_STARTING_COINS);
    assert_eq!(resolve_starting_coins(Some("12.5")), DEFAULT_STARTING_COINS);
}

#[test]
fn starting_coins_accepts_negative() {
    // Meme si pas tres utile, on n'applique pas de floor ici — c'est la
    // responsabilite du repo/handler si besoin.
    assert_eq!(resolve_starting_coins(Some("-50")), -50);
}

// ── validate_positive_amount ──

#[test]
fn positive_amount_accepts_gt_zero() {
    assert!(validate_positive_amount(1).is_ok());
    assert!(validate_positive_amount(1000).is_ok());
    assert!(validate_positive_amount(i64::MAX).is_ok());
}

#[test]
fn positive_amount_rejects_zero_and_negative() {
    assert_eq!(
        validate_positive_amount(0).unwrap_err(),
        "Le montant doit etre positif"
    );
    assert_eq!(
        validate_positive_amount(-1).unwrap_err(),
        "Le montant doit etre positif"
    );
    assert_eq!(
        validate_positive_amount(i64::MIN).unwrap_err(),
        "Le montant doit etre positif"
    );
}

// ── validate_transfer_distinct_users ──

#[test]
fn transfer_distinct_accepts_different_users() {
    assert!(validate_transfer_distinct_users("alice", "bob").is_ok());
}

#[test]
fn transfer_distinct_rejects_same_user() {
    assert_eq!(
        validate_transfer_distinct_users("alice", "alice").unwrap_err(),
        "Impossible de transferer vers soi-meme"
    );
}

#[test]
fn transfer_distinct_empty_users_are_equal() {
    // Cas limite : deux chaines vides -> meme user
    assert!(validate_transfer_distinct_users("", "").is_err());
}

// ── resolve_reset_balance ──

#[test]
fn reset_balance_none_uses_default() {
    assert_eq!(resolve_reset_balance(None), DEFAULT_STARTING_COINS);
}

#[test]
fn reset_balance_preserves_positive() {
    assert_eq!(resolve_reset_balance(Some(500)), 500);
    assert_eq!(resolve_reset_balance(Some(0)), 0);
}

#[test]
fn reset_balance_floors_negative_to_zero() {
    assert_eq!(resolve_reset_balance(Some(-100)), 0);
    assert_eq!(resolve_reset_balance(Some(i64::MIN)), 0);
}
