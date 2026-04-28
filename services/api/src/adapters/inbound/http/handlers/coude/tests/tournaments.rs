use super::*;
use chrono::TimeZone;
use chrono::Utc;
fn ts() -> chrono::DateTime<chrono::Utc> {
    Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
}

// ── StandingDto ──

#[test]
fn standing_dto_serializes() {
    let s = StandingDto {
        user_id: "u".into(),
        username: "Alice".into(),
        net_gain: 1500,
        rank: 1,
    };
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("\"net_gain\":1500"));
    assert!(json.contains("\"rank\":1"));
    assert!(json.contains("\"username\":\"Alice\""));
}

#[test]
fn standing_dto_handles_negative_net_gain() {
    let s = StandingDto {
        user_id: "u".into(),
        username: "Bob".into(),
        net_gain: -500,
        rank: 10,
    };
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("\"net_gain\":-500"));
}

// ── CurrentTournamentDto ──

#[test]
fn current_tournament_dto_with_standings() {
    let t = CurrentTournamentDto {
        guild_id: "g".into(),
        week_start: ts(),
        week_end: ts(),
        prize_pool_estimated: 10000,
        standings: vec![
            StandingDto { user_id: "u1".into(), username: "A".into(), net_gain: 100, rank: 1 },
            StandingDto { user_id: "u2".into(), username: "B".into(), net_gain: 50, rank: 2 },
        ],
    };
    let json = serde_json::to_string(&t).unwrap();
    assert!(json.contains("\"prize_pool_estimated\":10000"));
    assert!(json.contains("\"standings\":["));
    // Verifie que les 2 standings sont presents
    assert!(json.contains("\"u1\""));
    assert!(json.contains("\"u2\""));
}

#[test]
fn current_tournament_dto_empty_standings() {
    let t = CurrentTournamentDto {
        guild_id: "g".into(),
        week_start: ts(),
        week_end: ts(),
        prize_pool_estimated: 0,
        standings: vec![],
    };
    let json = serde_json::to_string(&t).unwrap();
    assert!(json.contains("\"standings\":[]"));
    assert!(json.contains("\"prize_pool_estimated\":0"));
}

// ── PastTournamentDto ──

#[test]
fn past_tournament_dto_with_winner() {
    let t = PastTournamentDto {
        id: "uuid".into(),
        guild_id: "g".into(),
        week_start: ts(),
        week_end: ts(),
        winner_user_id: Some("u1".into()),
        winner_username: Some("Champion".into()),
        winner_net_gain: 5000,
        prize_amount: 10000,
        status: "resolved".into(),
        resolved_at: Some(ts()),
    };
    let json = serde_json::to_string(&t).unwrap();
    assert!(json.contains("\"winner_user_id\":\"u1\""));
    assert!(json.contains("\"winner_username\":\"Champion\""));
    assert!(json.contains("\"prize_amount\":10000"));
    assert!(json.contains("\"status\":\"resolved\""));
}

#[test]
fn past_tournament_dto_unresolved() {
    let t = PastTournamentDto {
        id: "uuid".into(),
        guild_id: "g".into(),
        week_start: ts(),
        week_end: ts(),
        winner_user_id: None,
        winner_username: None,
        winner_net_gain: 0,
        prize_amount: 0,
        status: "pending".into(),
        resolved_at: None,
    };
    let json = serde_json::to_string(&t).unwrap();
    assert!(json.contains("\"winner_user_id\":null"));
    assert!(json.contains("\"winner_username\":null"));
    assert!(json.contains("\"resolved_at\":null"));
    assert!(json.contains("\"status\":\"pending\""));
}
