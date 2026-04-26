use super::*;

#[test]
fn triggers_below_50c() {
    assert!(should_trigger(0));
    assert!(should_trigger(49));
    assert!(should_trigger(-100));
}

#[test]
fn does_not_trigger_at_or_above_50c() {
    assert!(!should_trigger(50));
    assert!(!should_trigger(100));
    assert!(!should_trigger(10_000));
}

#[test]
fn reduce_loss_no_net_returns_nominal() {
    assert_eq!(reduce_loss(1000, false), 1000);
    assert_eq!(reduce_loss(0, false), 0);
}

#[test]
fn reduce_loss_with_net_halves() {
    assert_eq!(reduce_loss(1000, true), 500);
    assert_eq!(reduce_loss(2, true), 1);
    assert_eq!(reduce_loss(1, true), 0); // arrondi vers 0
}

#[test]
fn reduce_loss_zero_or_negative_unchanged() {
    assert_eq!(reduce_loss(0, true), 0);
    assert_eq!(reduce_loss(-50, true), -50);
}

#[test]
fn boost_bet_gain_no_net_returns_nominal() {
    assert_eq!(boost_bet_gain(1000, false), 1000);
}

#[test]
fn boost_bet_gain_with_net_adds_50_percent() {
    assert_eq!(boost_bet_gain(1000, true), 1500);
    assert_eq!(boost_bet_gain(100, true), 150);
}

#[test]
fn boost_bet_gain_zero_unchanged() {
    assert_eq!(boost_bet_gain(0, true), 0);
    assert_eq!(boost_bet_gain(-100, true), -100);
}

#[test]
fn constants_match_spec() {
    assert_eq!(SAFETY_NET_TRIGGER_COINS, 50);
    assert_eq!(SAFETY_NET_DURATION_HOURS, 72);
    assert_eq!(SAFETY_NET_LOSS_MULTIPLIER, 0.5);
    assert_eq!(SAFETY_NET_BET_GAIN_MULTIPLIER, 1.5);
}

#[test]
fn is_active_at_respects_expiry() {
    use chrono::Duration;
    let now = chrono::Utc::now();
    let net = ActiveSafetyNet {
        id: uuid::Uuid::new_v4(),
        guild_id: "g".into(),
        user_id: "u".into(),
        activated_at: now,
        expires_at: now + Duration::hours(72),
    };
    assert!(net.is_active_at(now));
    assert!(net.is_active_at(now + Duration::hours(71)));
    assert!(!net.is_active_at(now + Duration::hours(73)));
}
