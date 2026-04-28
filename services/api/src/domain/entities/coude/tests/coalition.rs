use super::*;

#[test]
fn status_round_trips() {
    for s in [
        CoalitionStatus::Forming,
        CoalitionStatus::Active,
        CoalitionStatus::Broken,
        CoalitionStatus::Expired,
    ] {
        assert_eq!(CoalitionStatus::from_db_str(s.as_db_str()), Some(s));
    }
}

#[test]
fn unknown_status_returns_none() {
    assert_eq!(CoalitionStatus::from_db_str("foo"), None);
}

#[test]
fn constants_match_spec() {
    assert_eq!(COALITION_COST_PER_MEMBER, 500);
    assert_eq!(COALITION_MIN_MEMBERS, 3);
    assert_eq!(COALITION_DURATION_HOURS, 48);
    assert_eq!(COALITION_GAIN_MULTIPLIER, 0.80);
}

#[test]
fn apply_coalition_penalty_neutralizes_when_inactive() {
    assert_eq!(apply_coalition_penalty(1000, false), 1000);
    assert_eq!(apply_coalition_penalty(50, false), 50);
}

#[test]
fn apply_coalition_penalty_reduces_20_percent() {
    assert_eq!(apply_coalition_penalty(1000, true), 800);
    assert_eq!(apply_coalition_penalty(100, true), 80);
}

#[test]
fn apply_coalition_penalty_zero_or_negative_unchanged() {
    assert_eq!(apply_coalition_penalty(0, true), 0);
    assert_eq!(apply_coalition_penalty(-50, true), -50);
}

fn mk_coalition(status: CoalitionStatus, n_members: usize) -> ActiveCoalition {
    let now = Utc::now();
    ActiveCoalition {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        target_id: "t".into(),
        opened_at: now,
        expires_at: now + chrono::Duration::hours(48),
        status,
        broken_by: None,
        broken_at: None,
        members: (0..n_members)
            .map(|i| CoalitionMember {
                member_id: format!("m{i}"),
                member_name: format!("Member{i}"),
                joined_at: now,
            })
            .collect(),
    }
}

#[test]
fn should_become_active_at_3_members() {
    let c = mk_coalition(CoalitionStatus::Forming, 2);
    assert!(!c.should_become_active());
    let c = mk_coalition(CoalitionStatus::Forming, 3);
    assert!(c.should_become_active());
    let c = mk_coalition(CoalitionStatus::Forming, 5);
    assert!(c.should_become_active());
}

#[test]
fn should_not_re_activate_already_active() {
    let c = mk_coalition(CoalitionStatus::Active, 3);
    assert!(!c.should_become_active());
}

#[test]
fn is_active_at_respects_status_and_expiry() {
    let now = Utc::now();
    let c = mk_coalition(CoalitionStatus::Active, 3);
    assert!(c.is_active_at(now));
    let c_broken = mk_coalition(CoalitionStatus::Broken, 3);
    assert!(!c_broken.is_active_at(now));
}
