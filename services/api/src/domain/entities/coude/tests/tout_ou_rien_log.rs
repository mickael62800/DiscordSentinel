use super::*;

#[test]
fn outcome_round_trips() {
    for o in [ToutOuRienLogOutcome::Won, ToutOuRienLogOutcome::Lost] {
        assert_eq!(ToutOuRienLogOutcome::from_db_str(o.as_db_str()), Some(o));
    }
}

#[test]
fn outcome_unknown_returns_none() {
    assert_eq!(ToutOuRienLogOutcome::from_db_str("foo"), None);
    assert_eq!(ToutOuRienLogOutcome::from_db_str(""), None);
}
