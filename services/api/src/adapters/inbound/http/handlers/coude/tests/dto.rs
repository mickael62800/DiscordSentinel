//! Tests pour les conversions domain -> DTO + deserialization des DTOs
//! request du module Coup de Coude.

use super::*;
use crate::domain::entities::*;
use crate::domain::value_objects::CoudeClass;
use chrono::Utc;
use uuid::Uuid;

fn sample_player() -> CoudePlayer {
    let now = Utc::now();
    CoudePlayer {
        guild_id: "g".into(), user_id: "u1".into(), username: "alice".into(),
        coins: 500,
        total_wins: 5, total_losses: 2, total_draws: 1,
        total_earned: 2000, total_lost: 500, total_stolen: 100,
        cowardice_count: 1, chaos_events: 3, casino_wins: 10, casino_losses: 5,
        level: 8, xp: 1500, stat_points: 2, atk: 5, def: 3,
        class: Some(CoudeClass::Tank), title: Some("Guerrier".into()), class_changed_at: None,
        hp_current: 80, hp_max: 100, hp_last_regen: None, repos_last_used: None,
        season: 2, created_at: now, updated_at: now,
    }
}

// ── PlayerDto ──

#[test]
fn player_dto_from_ref_maps_all_fields() {
    let p = sample_player();
    let dto = PlayerDto::from(&p);
    assert_eq!(dto.user_id, "u1");
    assert_eq!(dto.username, "alice");
    assert_eq!(dto.coins, 500);
    assert_eq!(dto.total_wins, 5);
    assert_eq!(dto.total_losses, 2);
    assert_eq!(dto.level, 8);
    assert_eq!(dto.xp, 1500);
    assert_eq!(dto.class.as_deref(), Some("tank"));
    assert_eq!(dto.title.as_deref(), Some("Guerrier"));
}

#[test]
fn player_dto_none_class_and_title() {
    let mut p = sample_player();
    p.class = None;
    p.title = None;
    let dto = PlayerDto::from(&p);
    assert!(dto.class.is_none());
    assert!(dto.title.is_none());
}

#[test]
fn player_dto_serializes_to_json() {
    let p = sample_player();
    let dto = PlayerDto::from(&p);
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"coins\":500"));
    assert!(json.contains("\"class\":\"tank\""));
}

// ── GetOrCreatePlayerDto ──

#[test]
fn get_or_create_player_deserializes() {
    let raw = r#"{"user_id":"u1","username":"Alice"}"#;
    let dto: GetOrCreatePlayerDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.user_id, "u1");
    assert_eq!(dto.username, "Alice");
}

// ── UpdateClassDto ──

#[test]
fn update_class_dto_deserializes() {
    let dto: UpdateClassDto = serde_json::from_str(r#"{"class":"tank"}"#).unwrap();
    assert_eq!(dto.class, "tank");
}

// ── AddXpDto ──

#[test]
fn add_xp_dto_deserializes() {
    let dto: AddXpDto = serde_json::from_str(r#"{"amount":100}"#).unwrap();
    assert_eq!(dto.amount, 100);
}

// ── SpendStatDto / ResetStatsDto ──

#[test]
fn spend_stat_dto_deserializes() {
    let dto: SpendStatDto = serde_json::from_str(r#"{"stat":"atk"}"#).unwrap();
    assert_eq!(dto.stat, "atk");
}

#[test]
fn reset_stats_dto_deserializes() {
    let dto: ResetStatsDto = serde_json::from_str(r#"{"cost":500}"#).unwrap();
    assert_eq!(dto.cost, 500);
}

// ── Record{Win,Loss,Draw}Dto ──

#[test]
fn record_win_dto_deserializes() {
    let dto: RecordWinDto = serde_json::from_str(r#"{"earned":100,"stolen":50}"#).unwrap();
    assert_eq!(dto.earned, 100);
    assert_eq!(dto.stolen, 50);
}

#[test]
fn record_loss_dto_deserializes() {
    let dto: RecordLossDto = serde_json::from_str(r#"{"lost":200}"#).unwrap();
    assert_eq!(dto.lost, 200);
}

#[test]
fn record_draw_dto_deserializes() {
    let dto: RecordDrawDto = serde_json::from_str(r#"{"lost":10}"#).unwrap();
    assert_eq!(dto.lost, 10);
}

// ── AmountDto / UpdateHpDto ──

#[test]
fn amount_dto_deserializes() {
    let dto: AmountDto = serde_json::from_str(r#"{"amount":999}"#).unwrap();
    assert_eq!(dto.amount, 999);
}

#[test]
fn update_hp_dto_deserializes() {
    let dto: UpdateHpDto = serde_json::from_str(r#"{"hp_current":50,"hp_max":100}"#).unwrap();
    assert_eq!(dto.hp_current, 50);
    assert_eq!(dto.hp_max, 100);
}

// ── CombatDto ──

#[test]
fn combat_dto_from_domain() {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let c = CoudeCombat {
        id, guild_id: "g".into(),
        channel_id: Some("c1".into()),
        attacker_id: "a".into(), attacker_name: "A".into(),
        defender_id: "d".into(), defender_name: "D".into(),
        mise: 100,
        status: "pending".into(),
        winner_id: None,
        attacker_roll: None, defender_roll: None,
        chaos_event: None, special_attack: None, defender_special: None,
        coins_transferred: None, result_message: None, message_id: None,
        created_at: now, accepted_at: None, resolved_at: None,
    };
    let dto = CombatDto::from(&c);
    assert_eq!(dto.id, id.to_string());
    assert_eq!(dto.attacker_id, "a");
    assert_eq!(dto.defender_name, "D");
    assert_eq!(dto.mise, 100);
    assert_eq!(dto.status, "pending");
}

// ── CreateCombatDto ──

#[test]
fn create_combat_dto_deserializes() {
    let raw = r#"{"channel_id":"c1","attacker_id":"a","attacker_name":"A",
                  "defender_id":"d","defender_name":"D","mise":100,"special_attack":null}"#;
    let dto: CreateCombatDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.attacker_id, "a");
    assert_eq!(dto.mise, 100);
}

// ── BetDto ──

#[test]
fn bet_dto_from_domain() {
    let id = Uuid::new_v4();
    let cid = Uuid::new_v4();
    let b = CoudeBet {
        id,
        guild_id: "g".into(),
        combat_id: cid,
        bettor_id: "u".into(),
        bettor_name: "Joe".into(),
        backed_id: "a".into(),
        amount: 100,
        won: Some(true),
        payout: Some(250),
    };
    let dto = BetDto::from(&b);
    assert_eq!(dto.id, id.to_string());
    assert_eq!(dto.combat_id, cid.to_string());
    assert_eq!(dto.amount, 100);
    assert_eq!(dto.bettor_name, "Joe");
}

// ── PlaceBetDto ──

#[test]
fn place_bet_dto_deserializes() {
    let raw = r#"{"combat_id":"00000000-0000-0000-0000-000000000000","bettor_id":"u","bettor_name":"Joe","backed_id":"a","amount":100}"#;
    let dto: PlaceBetDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.amount, 100);
    assert_eq!(dto.bettor_name, "Joe");
}

// ── StealDto + TransferCoinsDto ──

#[test]
fn steal_dto_deserializes() {
    let raw = r#"{"thief_id":"t","victim_id":"v","amount":100}"#;
    let dto: StealDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.thief_id, "t");
    assert_eq!(dto.victim_id, "v");
    assert_eq!(dto.amount, 100);
}

#[test]
fn transfer_coins_dto_deserializes() {
    let raw = r#"{"from_id":"f","to_id":"t","amount":500}"#;
    let dto: TransferCoinsDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.from_id, "f");
    assert_eq!(dto.to_id, "t");
    assert_eq!(dto.amount, 500);
}

// ── InventoryItemDto ──

#[test]
fn inventory_item_dto_from_domain() {
    let item = CoudeInventoryItem {
        guild_id: "g".into(),
        user_id: "u".into(),
        item_key: "potion".into(),
        quantity: 5,
    };
    let dto = InventoryItemDto::from(item);
    assert_eq!(dto.item_key, "potion");
    assert_eq!(dto.quantity, 5);
}

#[test]
fn add_item_dto_deserializes() {
    let dto: AddItemDto = serde_json::from_str(r#"{"item_key":"potion"}"#).unwrap();
    assert_eq!(dto.item_key, "potion");
}

// ── EventDto ──

#[test]
fn event_dto_from_domain() {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let e = CoudeEvent {
        id, guild_id: "g".into(),
        event_type: "happy_hour".into(),
        active: true,
        expires_at: now, created_at: now,
    };
    let dto = EventDto::from(e);
    assert_eq!(dto.id, id.to_string());
    assert_eq!(dto.guild_id, "g");
    assert!(dto.active);
}

// ── CurrentSeasonDto ──

#[test]
fn current_season_dto_from_domain() {
    let now = Utc::now();
    let s = CoudeCurrentSeason {
        season_number: 3,
        started_at: now, ends_at: now,
        days_remaining: 15,
    };
    let dto = CurrentSeasonDto::from(s);
    assert_eq!(dto.season_number, 3);
    assert_eq!(dto.days_remaining, 15);
}
