use super::*;
use chrono::Duration;

#[test]
fn status_round_trips() {
    for s in [
        VendettaStatus::Active,
        VendettaStatus::Won,
        VendettaStatus::Lost,
        VendettaStatus::Expired,
    ] {
        assert_eq!(VendettaStatus::from_db_str(s.as_db_str()), Some(s));
    }
}

#[test]
fn unknown_status_returns_none() {
    assert_eq!(VendettaStatus::from_db_str("foo"), None);
}

#[test]
fn revenge_bonus_doubles_nominal_when_active() {
    assert_eq!(apply_revenge_bonus(1000, true), 2000);
    assert_eq!(apply_revenge_bonus(50, true), 100);
}

#[test]
fn revenge_bonus_unchanged_when_inactive() {
    assert_eq!(apply_revenge_bonus(1000, false), 1000);
}

#[test]
fn revenge_bonus_zero_unchanged() {
    assert_eq!(apply_revenge_bonus(0, true), 0);
    assert_eq!(apply_revenge_bonus(-100, true), -100);
}

#[test]
fn is_active_at_true_when_pending_and_within_window() {
    let now = Utc::now();
    let v = ActiveVendetta {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        challenger_id: "c".into(),
        target_id: "t".into(),
        declared_at: now,
        expires_at: now + Duration::hours(168),
        status: VendettaStatus::Active,
        resolved_at: None,
    };
    assert!(v.is_active_at(now));
    assert!(v.is_active_at(now + Duration::hours(167)));
}

#[test]
fn is_active_at_false_when_resolved() {
    let now = Utc::now();
    let v = ActiveVendetta {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        challenger_id: "c".into(),
        target_id: "t".into(),
        declared_at: now,
        expires_at: now + Duration::hours(168),
        status: VendettaStatus::Won,
        resolved_at: Some(now),
    };
    assert!(!v.is_active_at(now));
}

#[test]
fn is_active_at_false_when_expired() {
    let now = Utc::now();
    let v = ActiveVendetta {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        challenger_id: "c".into(),
        target_id: "t".into(),
        declared_at: now - Duration::hours(200),
        expires_at: now - Duration::hours(1),
        status: VendettaStatus::Active,
        resolved_at: None,
    };
    assert!(!v.is_active_at(now));
}

#[test]
fn constants_match_spec() {
    assert_eq!(VENDETTA_WINDOW_HOURS, 168);
    assert_eq!(VENDETTA_WIN_BONUS_MULTIPLIER, 2.0);
}
