//! Tests pour les conversions domain -> DTO + deserialization des DTOs
//! request du module Coup de Coude.

use super::*;
use crate::domain::entities::coude::bet::BetPayout;
use crate::domain::entities::coude::bet::BetResolutionPlan;
use crate::domain::entities::coude::bet::CoudeBet;
use crate::domain::entities::coude::bet::FighterBetBonus as CoudeFighterBetBonus;
use crate::domain::entities::coude::combat::*;
use crate::domain::entities::coude::inventory::*;
use crate::domain::entities::coude::player::*;
use crate::domain::entities::coude::social::*;
use crate::domain::enums::coude::coude_class::CoudeClass;
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

// ── FullPlayerDto ──

#[test]
fn full_player_dto_from_domain_maps_all_fields() {
    let p = sample_player();
    let dto = FullPlayerDto::from(p);
    assert_eq!(dto.guild_id, "g");
    assert_eq!(dto.user_id, "u1");
    assert_eq!(dto.class.as_deref(), Some("tank"));
    assert_eq!(dto.hp_current, 80);
    assert_eq!(dto.hp_max, 100);
    assert_eq!(dto.season, 2);
    assert!(dto.created_at.contains('T'));
}

#[test]
fn full_player_dto_preserves_none_optional_dates() {
    let p = sample_player();
    let dto = FullPlayerDto::from(p);
    assert!(dto.hp_last_regen.is_none());
    assert!(dto.repos_last_used.is_none());
    assert!(dto.class_changed_at.is_none());
}

// ── FullCombatDto ──

fn sample_combat() -> CoudeCombat {
    let now = Utc::now();
    CoudeCombat {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: Some("c1".into()),
        attacker_id: "a".into(), attacker_name: "A".into(),
        defender_id: "d".into(), defender_name: "D".into(),
        mise: 100, status: "betting".into(),
        winner_id: None, attacker_roll: None, defender_roll: None,
        chaos_event: None, special_attack: None, defender_special: None,
        coins_transferred: None, result_message: None,
        message_id: Some("msg".into()),
        created_at: now, accepted_at: None, resolved_at: None,
    }
}

#[test]
fn full_combat_dto_from_domain_maps_all_fields() {
    let c = sample_combat();
    let id = c.id.to_string();
    let dto = FullCombatDto::from(c);
    assert_eq!(dto.id, id);
    assert_eq!(dto.channel_id.as_deref(), Some("c1"));
    assert_eq!(dto.message_id.as_deref(), Some("msg"));
    assert_eq!(dto.mise, 100);
    assert_eq!(dto.status, "betting");
}

// ── PrimeDto ──

#[test]
fn prime_dto_from_domain_maps_all_fields() {
    let now = Utc::now();
    let p = CoudePrime {
        id: Uuid::new_v4(), guild_id: "g".into(),
        target_id: "t".into(), target_name: "T".into(),
        placed_by_id: "p".into(), placed_by_name: "P".into(),
        amount: 500, claimed: false,
        claimed_by_id: None, claimed_by_name: None, claimed_at: None,
        created_at: now,
    };
    let dto = PrimeDto::from(p);
    assert_eq!(dto.amount, 500);
    assert!(!dto.claimed);
    assert!(dto.claimed_at.is_none());
    assert!(dto.created_at.contains('T'));
}

#[test]
fn prime_dto_claimed_carries_metadata() {
    let now = Utc::now();
    let p = CoudePrime {
        id: Uuid::new_v4(), guild_id: "g".into(),
        target_id: "t".into(), target_name: "T".into(),
        placed_by_id: "p".into(), placed_by_name: "P".into(),
        amount: 500, claimed: true,
        claimed_by_id: Some("c".into()),
        claimed_by_name: Some("C".into()),
        claimed_at: Some(now),
        created_at: now,
    };
    let dto = PrimeDto::from(p);
    assert!(dto.claimed);
    assert_eq!(dto.claimed_by_id.as_deref(), Some("c"));
    assert!(dto.claimed_at.is_some());
}

// ── InsuranceDto ──

#[test]
fn insurance_dto_from_domain() {
    let now = Utc::now();
    let i = CoudeInsurance {
        id: Uuid::new_v4(),
        is_scam: true,
        expires_at: now,
    };
    let _ = now;
    let id = i.id.to_string();
    let dto = InsuranceDto::from(i);
    assert_eq!(dto.id, id);
    assert!(dto.is_scam);
    assert!(dto.expires_at.contains('T'));
}

// ── AddXpResponse ──

#[test]
fn add_xp_response_from_xp_progress() {
    let p = XpProgress {
        new_xp: 1500, new_level: 9, leveled_up: true, stat_points_gained: 2,
    };
    let r: AddXpResponse = p.into();
    assert_eq!(r.new_xp, 1500);
    assert_eq!(r.new_level, 9);
    assert!(r.leveled_up);
    assert_eq!(r.stat_points_gained, 2);
}

// ── LeaderboardEntry ──

#[test]
fn leaderboard_entry_from_domain() {
    let e = CoudeLeaderboardEntry {
        user_id: "u".into(), username: "Alice".into(), value: 12345,
    };
    let dto: LeaderboardEntry = e.into();
    assert_eq!(dto.user_id, "u");
    assert_eq!(dto.value, 12345);
}

// ── FighterBetBonus ──

#[test]
fn fighter_bet_bonus_from_domain() {
    let b = CoudeFighterBetBonus {
        winner_id: "w".into(), winner_bonus: 100,
        loser_id: "l".into(), loser_bonus: 50,
        total_pot: 200,
    };
    let dto: FighterBetBonus = b.into();
    assert_eq!(dto.winner_bonus, 100);
    assert_eq!(dto.loser_bonus, 50);
    assert_eq!(dto.total_pot, 200);
}

// ── ResolveBetsResponse from BetResolutionPlan ──

#[test]
fn resolve_bets_response_maps_payouts_and_bonus() {
    let plan = BetResolutionPlan {
        payouts: vec![BetPayout {
            bet_id: Uuid::new_v4(),
            bettor_id: "u1".into(), bettor_name: "U1".into(),
            backed_id: "att".into(), amount_bet: 100,
            payout: 200, won: true,
        }],
        fighter_bonus: Some(CoudeFighterBetBonus {
            winner_id: "att".into(), winner_bonus: 50,
            loser_id: "def".into(), loser_bonus: 0,
            total_pot: 100,
        }),
    };
    let resp: ResolveBetsResponse = plan.into();
    assert_eq!(resp.results.len(), 1);
    assert_eq!(resp.results[0].payout, 200);
    assert!(resp.results[0].won);
    assert!(resp.fighter_bonus.is_some());
    assert!(resp.taunt_events.is_empty());
}

#[test]
fn resolve_bets_response_empty_plan() {
    let plan = BetResolutionPlan { payouts: vec![], fighter_bonus: None };
    let resp: ResolveBetsResponse = plan.into();
    assert!(resp.results.is_empty());
    assert!(resp.fighter_bonus.is_none());
}

// ── CombatQueryParams / LeaderboardQueryParams ──

#[test]
fn combat_query_params_deserializes() {
    let q: CombatQueryParams = serde_json::from_value(
        serde_json::json!({"status": "resolved", "limit": 25})
    ).unwrap();
    assert_eq!(q.status.as_deref(), Some("resolved"));
    assert_eq!(q.limit, Some(25));

    let empty: CombatQueryParams = serde_json::from_str("{}").unwrap();
    assert!(empty.status.is_none());
    assert!(empty.limit.is_none());
}

#[test]
fn leaderboard_query_params_deserializes() {
    let q: LeaderboardQueryParams = serde_json::from_value(
        serde_json::json!({"limit": 50})
    ).unwrap();
    assert_eq!(q.limit, Some(50));
    let empty: LeaderboardQueryParams = serde_json::from_str("{}").unwrap();
    assert!(empty.limit.is_none());
}

// ── BuyInsuranceDto ──

#[test]
fn buy_insurance_dto_default_duration_zero() {
    let dto: BuyInsuranceDto = serde_json::from_value(
        serde_json::json!({"user_id": "u", "is_scam": true})
    ).unwrap();
    assert_eq!(dto.user_id, "u");
    assert!(dto.is_scam);
    // Default est 0 avec #[serde(default)] (i64::default()).
    assert_eq!(dto.duration_seconds, 0);
}

#[test]
fn buy_insurance_dto_with_duration() {
    let dto: BuyInsuranceDto = serde_json::from_value(
        serde_json::json!({"user_id": "u", "is_scam": false, "duration_seconds": 7200})
    ).unwrap();
    assert_eq!(dto.duration_seconds, 7200);
}

// ── CreatePrimeDto / ClaimPrimesDto / UseItemDto ──

#[test]
fn create_prime_dto_deserializes() {
    let dto: CreatePrimeDto = serde_json::from_value(
        serde_json::json!({
            "target_id": "t", "target_name": "T",
            "placed_by_id": "p", "placed_by_name": "P",
            "amount": 500
        })
    ).unwrap();
    assert_eq!(dto.amount, 500);
    assert_eq!(dto.target_id, "t");
}

#[test]
fn claim_primes_dto_deserializes() {
    let dto: ClaimPrimesDto = serde_json::from_value(
        serde_json::json!({
            "target_id": "t", "claimer_id": "c", "claimer_name": "C"
        })
    ).unwrap();
    assert_eq!(dto.target_id, "t");
    assert_eq!(dto.claimer_name, "C");
}

#[test]
fn use_item_dto_deserializes() {
    let dto: UseItemDto = serde_json::from_value(
        serde_json::json!({"item_key": "potion"})
    ).unwrap();
    assert_eq!(dto.item_key, "potion");
}

#[test]
fn defender_special_dto_deserializes() {
    let dto: DefenderSpecialDto = serde_json::from_value(
        serde_json::json!({"item_key": "fake_plaque"})
    ).unwrap();
    assert_eq!(dto.item_key, "fake_plaque");
}

#[test]
fn set_betting_dto_deserializes() {
    let dto: SetBettingDto = serde_json::from_value(
        serde_json::json!({"message_id": "123"})
    ).unwrap();
    assert_eq!(dto.message_id, "123");
}

#[test]
fn resolve_combat_dto_all_optional_except_status() {
    let dto: ResolveCombatDto = serde_json::from_value(
        serde_json::json!({"status": "resolved"})
    ).unwrap();
    assert_eq!(dto.status, "resolved");
    assert!(dto.winner_id.is_none());
    assert!(dto.coins_transferred.is_none());
}

#[test]
fn gain_lost_dtos_deserialize() {
    let g: GainDto = serde_json::from_value(serde_json::json!({"gain": 500})).unwrap();
    assert_eq!(g.gain, 500);
    let l: LostDto = serde_json::from_value(serde_json::json!({"lost": 300})).unwrap();
    assert_eq!(l.lost, 300);
}

#[test]
fn daily_chaos_dto_deserializes() {
    let dto: DailyChaosDto = serde_json::from_value(serde_json::json!({
        "loser_id": "l", "loser_name": "L",
        "winner_id": "w", "winner_name": "W",
        "amount": 500,
    })).unwrap();
    assert_eq!(dto.amount, 500);
}

#[test]
fn duration_dto_deserializes() {
    let d: DurationDto = serde_json::from_value(serde_json::json!({"duration_secs": 3600})).unwrap();
    assert_eq!(d.duration_secs, 3600);
}

#[test]
fn random_players_query_default_none() {
    let q: RandomPlayersQuery = serde_json::from_str("{}").unwrap();
    assert!(q.count.is_none());
    let q2: RandomPlayersQuery = serde_json::from_value(serde_json::json!({"count": 5})).unwrap();
    assert_eq!(q2.count, Some(5));
}

#[test]
fn adjust_coins_dto_deserializes() {
    let d: AdjustCoinsDto = serde_json::from_value(serde_json::json!({"amount": -100})).unwrap();
    assert_eq!(d.amount, -100);
}
