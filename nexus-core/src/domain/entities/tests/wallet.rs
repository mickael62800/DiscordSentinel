use crate::domain::entities::wallet::Wallet;

#[test]
fn new_wallet_starts_at_zero() {
    let w = Wallet::new("g1", "u1");
    assert_eq!(w.coins, 0);
    assert_eq!(w.total_earned, 0);
    assert_eq!(w.total_spent, 0);
}

#[test]
fn credit_increases_coins_and_total_earned() {
    let mut w = Wallet::new("g1", "u1");
    w.credit(500).unwrap();
    w.credit(200).unwrap();
    assert_eq!(w.coins, 700);
    assert_eq!(w.total_earned, 700);
    assert_eq!(w.total_spent, 0);
}

#[test]
fn credit_rejects_zero_and_negative() {
    let mut w = Wallet::new("g1", "u1");
    assert!(w.credit(0).is_err());
    assert!(w.credit(-10).is_err());
    assert_eq!(w.coins, 0);
}

#[test]
fn debit_removes_coins_and_tracks_total_spent() {
    let mut w = Wallet::new("g1", "u1");
    w.credit(1000).unwrap();
    let actual = w.debit_clamped(300).unwrap();
    assert_eq!(actual, 300);
    assert_eq!(w.coins, 700);
    assert_eq!(w.total_spent, 300);
}

#[test]
fn debit_is_clamped_to_balance_never_negative() {
    let mut w = Wallet::new("g1", "u1");
    w.credit(100).unwrap();
    let actual = w.debit_clamped(2000).unwrap();
    assert_eq!(actual, 100);
    assert_eq!(w.coins, 0);
    assert_eq!(w.total_spent, 100);
}

#[test]
fn debit_on_empty_wallet_debits_nothing() {
    let mut w = Wallet::new("g1", "u1");
    let actual = w.debit_clamped(500).unwrap();
    assert_eq!(actual, 0);
    assert_eq!(w.coins, 0);
    assert_eq!(w.total_spent, 0);
}

#[test]
fn debit_rejects_zero_and_negative() {
    let mut w = Wallet::new("g1", "u1");
    w.credit(100).unwrap();
    assert!(w.debit_clamped(0).is_err());
    assert!(w.debit_clamped(-5).is_err());
    assert_eq!(w.coins, 100);
}

#[test]
fn credit_saturates_instead_of_overflowing() {
    let mut w = Wallet::new("g1", "u1");
    w.coins = i64::MAX - 1;
    w.credit(1000).unwrap();
    assert_eq!(w.coins, i64::MAX);
}
