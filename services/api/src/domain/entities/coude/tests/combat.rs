use super::*;
use chrono::Utc;
use uuid::Uuid;

fn combat(status: &str) -> Combat {
    Combat {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: None,
        attacker_id: "a".into(),
        attacker_name: "A".into(),
        defender_id: "d".into(),
        defender_name: "D".into(),
        mise: 100,
        status: status.into(),
        winner_id: None,
        attacker_roll: None,
        defender_roll: None,
        chaos_event: None,
        special_attack: None,
        defender_special: None,
        coins_transferred: None,
        result_message: None,
        message_id: None,
        created_at: Utc::now(),
        accepted_at: None,
        resolved_at: None,
    }
}

#[test]
fn active_statuses_contains_expected_values() {
    assert!(Combat::ACTIVE_STATUSES.contains(&"pending"));
    assert!(Combat::ACTIVE_STATUSES.contains(&"accepted"));
    assert!(Combat::ACTIVE_STATUSES.contains(&"betting"));
    assert_eq!(Combat::ACTIVE_STATUSES.len(), 3);
}

#[test]
fn is_active_true_for_active_statuses() {
    assert!(combat("pending").is_active());
    assert!(combat("accepted").is_active());
    assert!(combat("betting").is_active());
}

#[test]
fn is_active_false_for_terminal_statuses() {
    assert!(!combat("resolved").is_active());
    assert!(!combat("expired").is_active());
    assert!(!combat("refused").is_active());
}

#[test]
fn is_active_false_for_unknown_status() {
    assert!(!combat("").is_active());
    assert!(!combat("unknown").is_active());
    assert!(!combat("PENDING").is_active()); // case-sensitive
}
