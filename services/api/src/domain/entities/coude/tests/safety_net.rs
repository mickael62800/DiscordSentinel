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
    // 1 * 0.5 = 0.5 -> round() = 1 (round-half-away-from-zero en Rust).
    // On reste sur cette convention pour eviter les pertes nulles abusives
    // sur micro-pertes : si tu perds 1c, tu en perds bien 1, pas 0.
    assert_eq!(reduce_loss(1, true), 1);
    // 3 * 0.5 = 1.5 -> 2 (avant fix : truncate = 1).
    assert_eq!(reduce_loss(3, true), 2);
    // 7 * 0.5 = 3.5 -> 4 (avant fix : truncate = 3).
    assert_eq!(reduce_loss(7, true), 4);
}

#[test]
fn reduce_loss_with_custom_multiplier_uses_round() {
    // Verifie le comportement parametrable utilise par GuildSettings
    // (cf. migration 170 — safety_net_loss_percent).
    assert_eq!(reduce_loss_with_multiplier(10, true, 0.7), 7);
    assert_eq!(reduce_loss_with_multiplier(10, true, 0.75), 8); // 7.5 -> 8
    assert_eq!(reduce_loss_with_multiplier(10, true, 0.0), 0);
    assert_eq!(reduce_loss_with_multiplier(10, true, 1.0), 10);
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
    // 5 * 1.5 = 7.5 -> 8 (avant fix : truncate = 7, le joueur perdait 1c).
    assert_eq!(boost_bet_gain(5, true), 8);
    assert_eq!(boost_bet_gain(3, true), 5); // 4.5 -> 5
}

#[test]
fn boost_bet_gain_with_custom_multiplier_uses_round() {
    // safety_net_bet_gain_percent configurable (default 150 = x1.5).
    assert_eq!(boost_bet_gain_with_multiplier(10, true, 1.25), 13); // 12.5 -> 13
    assert_eq!(boost_bet_gain_with_multiplier(10, true, 2.0), 20);
    assert_eq!(boost_bet_gain_with_multiplier(10, true, 1.0), 10);
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
