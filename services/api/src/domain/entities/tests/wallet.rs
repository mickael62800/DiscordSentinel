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
