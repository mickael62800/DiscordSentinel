use super::*;
use chrono::TimeZone;
use chrono::Utc;
use uuid::Uuid;

fn ts() -> chrono::DateTime<chrono::Utc> {
    Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
}

fn sample_game(status: &str) -> BlackjackGame {
    BlackjackGame {
        id: Uuid::nil(),
        guild_id: "g1".into(),
        user_id: "u1".into(),
        username: "alice".into(),
        bet: 100,
        player_hand: vec![
            Card { rank: "10".into(), suit: "hearts".into() },
            Card { rank: "As".into(), suit: "spades".into() },
        ],
        dealer_hand: vec![
            Card { rank: "King".into(), suit: "clubs".into() },
            Card { rank: "7".into(), suit: "diamonds".into() },
        ],
        deck: vec![],
        status: status.into(),
        player_score: 21,
        dealer_score: 17,
        doubled: false,
        payout: 150,
        created_at: ts(),
        finished_at: Some(ts()),
    }
}

// ── CardDto::from ──

#[test]
fn card_dto_from_includes_filename() {
    let card = Card { rank: "As".into(), suit: "hearts".into() };
    let dto = CardDto::from(&card);
    assert_eq!(dto.rank, "As");
    assert_eq!(dto.suit, "hearts");
    assert_eq!(dto.filename, "As_hearts.jpg");
}

// ── game_is_over (delegate to domain) ──

#[test]
fn game_is_over_terminal_states() {
    for s in &["player_blackjack", "player_bust", "dealer_bust",
               "player_win", "dealer_win", "push"] {
        assert!(game_is_over(s), "{s} doit etre terminal");
    }
}

#[test]
fn game_is_over_in_progress_states() {
    for s in &["playing", "in_progress", "", "unknown"] {
        assert!(!game_is_over(s), "{s} ne doit PAS etre terminal");
    }
}

// ── to_dto : révélation du dealer ──

#[test]
fn to_dto_finished_game_reveals_all_dealer_cards() {
    let dto = to_dto(&sample_game("player_win"));
    assert_eq!(dto.dealer_hand.len(), 2);
    assert_eq!(dto.dealer_hand[0].rank, "King");
    assert_eq!(dto.dealer_hand[1].rank, "7");
    assert_eq!(dto.dealer_score, 17);
}

#[test]
fn to_dto_in_progress_hides_dealer_second_card() {
    let dto = to_dto(&sample_game("playing"));
    assert_eq!(dto.dealer_hand.len(), 2);
    assert_eq!(dto.dealer_hand[0].rank, "King");
    assert_eq!(dto.dealer_hand[1].rank, "hidden");
    assert_eq!(dto.dealer_hand[1].suit, "hidden");
    assert_eq!(dto.dealer_hand[1].filename, "card_back.jpg");
}

#[test]
fn to_dto_in_progress_dealer_score_is_only_visible_card() {
    let dto = to_dto(&sample_game("playing"));
    // King = 10 (premiere carte seulement, pas 17)
    assert_eq!(dto.dealer_score, 10);
    assert_eq!(dto.player_score, 21);
}

#[test]
fn to_dto_in_progress_single_dealer_card_no_back() {
    // Edge case : dealer a une seule carte (distribution partielle).
    let mut g = sample_game("playing");
    g.dealer_hand = vec![Card { rank: "7".into(), suit: "diamonds".into() }];
    let dto = to_dto(&g);
    assert_eq!(dto.dealer_hand.len(), 1);
    assert_eq!(dto.dealer_hand[0].rank, "7");
}

#[test]
fn to_dto_empty_dealer_hand_produces_empty_dto_hand() {
    let mut g = sample_game("playing");
    g.dealer_hand = vec![];
    let dto = to_dto(&g);
    assert!(dto.dealer_hand.is_empty());
    assert_eq!(dto.dealer_score, 0);
}

#[test]
fn to_dto_player_hand_always_visible() {
    let dto_playing = to_dto(&sample_game("playing"));
    let dto_finished = to_dto(&sample_game("push"));
    assert_eq!(dto_playing.player_hand.len(), 2);
    assert_eq!(dto_finished.player_hand.len(), 2);
    assert_eq!(dto_playing.player_hand[0].rank, "10");
    assert_eq!(dto_playing.player_hand[1].rank, "As");
}

#[test]
fn to_dto_preserves_scalar_fields() {
    let dto = to_dto(&sample_game("push"));
    assert_eq!(dto.id, Uuid::nil().to_string());
    assert_eq!(dto.guild_id, "g1");
    assert_eq!(dto.user_id, "u1");
    assert_eq!(dto.username, "alice");
    assert_eq!(dto.bet, 100);
    assert_eq!(dto.payout, 150);
    assert!(!dto.doubled);
    assert_eq!(dto.created_at, ts().to_rfc3339());
    assert_eq!(dto.finished_at, Some(ts().to_rfc3339()));
}

#[test]
fn to_dto_unfinished_at_none_preserved() {
    let mut g = sample_game("playing");
    g.finished_at = None;
    let dto = to_dto(&g);
    assert!(dto.finished_at.is_none());
}

// ── Deserialization ──

#[test]
fn start_game_dto_deserializes() {
    let raw = r#"{"guild_id":"g","user_id":"u","username":"a","bet":100}"#;
    let dto: StartGameDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.guild_id, "g");
    assert_eq!(dto.bet, 100);
}

#[test]
fn create_table_dto_deserializes() {
    let raw = r#"{"guild_id":"g","channel_id":"c","owner_id":"o","owner_name":"O"}"#;
    let dto: CreateTableDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.channel_id, "c");
    assert_eq!(dto.owner_name, "O");
}

#[test]
fn join_table_dto_deserializes() {
    let raw = r#"{"user_id":"u","user_name":"U"}"#;
    let dto: JoinTableDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.user_id, "u");
}
