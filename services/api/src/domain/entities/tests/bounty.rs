use super::*;

#[test]
fn status_round_trips() {
    for s in [BountyStatus::Open, BountyStatus::Claimed, BountyStatus::Expired] {
        assert_eq!(BountyStatus::from_db_str(s.as_db_str()), Some(s));
    }
}

#[test]
fn unknown_status_returns_none() {
    assert_eq!(BountyStatus::from_db_str("foo"), None);
}

#[test]
fn constants_match_spec() {
    assert_eq!(BOUNTY_AUTO_OPEN_STREAK_THRESHOLD, 5);
    assert_eq!(BOUNTY_INITIAL_AMOUNT, 1000);
    assert!(BOUNTY_MIN_CONTRIBUTION > 0);
}
