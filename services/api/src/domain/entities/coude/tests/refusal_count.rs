use super::*;

#[test]
fn threshold_is_3() {
    assert_eq!(HONOR_DEBT_THRESHOLD, 3);
}

#[test]
fn honor_debt_owed_below_threshold() {
    let r = RefusalCount {
        guild_id: "g".into(),
        requester_id: "r".into(),
        refuser_id: "f".into(),
        count: 0,
        last_refused_at: Utc::now(),
    };
    assert!(!r.honor_debt_owed());

    let r2 = RefusalCount { count: 2, ..r.clone() };
    assert!(!r2.honor_debt_owed());
}

#[test]
fn honor_debt_owed_at_or_above_threshold() {
    let r = RefusalCount {
        guild_id: "g".into(),
        requester_id: "r".into(),
        refuser_id: "f".into(),
        count: 3,
        last_refused_at: Utc::now(),
    };
    assert!(r.honor_debt_owed());
    let r2 = RefusalCount { count: 5, ..r.clone() };
    assert!(r2.honor_debt_owed());
}
