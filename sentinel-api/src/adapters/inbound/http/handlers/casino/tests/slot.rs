//! Tests des conversions DTO du handler slot.

use chrono::Utc;
use uuid::Uuid;

use crate::adapters::inbound::http::handlers::casino::slot::SlotSpinDto;
use crate::adapters::inbound::http::handlers::casino::slot::SlotTopWinnerDto;
use crate::adapters::inbound::http::handlers::casino::slot::SpinResponseDto;
use crate::ports::inbound::casino::manage_slot::SpinResult;
use sentinel_core::domain::entities::casino::slot::SlotSpin;
use sentinel_core::domain::entities::casino::slot::SlotTopWinner;

fn sample_spin() -> SlotSpin {
    SlotSpin {
        id: Uuid::nil(),
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "Alice".into(),
        mise: 100,
        symbols: vec!["🍒".into(), "🍒".into(), "🍋".into()],
        payout: 100,
        multiplier: 1.0,
        is_jackpot: false,
        is_free: false,
        created_at: Utc::now(),
    }
}

#[test]
fn slot_spin_dto_from_entity() {
    let spin = sample_spin();
    let dto = SlotSpinDto::from(spin.clone());
    assert_eq!(dto.id, spin.id.to_string());
    assert_eq!(dto.user_id, "u".into());
    assert_eq!(dto.username, "Alice");
    assert_eq!(dto.mise, 100);
    assert_eq!(
        dto.symbols,
        vec!["🍒".to_string(), "🍒".to_string(), "🍋".to_string()]
    );
    assert_eq!(dto.payout, 100);
    assert!(!dto.is_jackpot);
    assert!(!dto.is_free);
}

#[test]
fn spin_response_dto_from_result_no_taunts() {
    let result = SpinResult {
        spin: sample_spin(),
        jackpot_pool_after: 5000,
        balance_after: 1234,
        triggered_taunts: vec![],
    };
    let dto = SpinResponseDto::from(result);
    assert_eq!(dto.spin_id, Uuid::nil().to_string());
    assert_eq!(dto.symbols.len(), 3);
    assert_eq!(dto.mise, 100);
    assert_eq!(dto.payout, 100);
    assert_eq!(dto.jackpot_pool_after, 5000);
    assert_eq!(dto.balance_after, 1234);
    assert_eq!(dto.triggered_taunts.len(), 0);
}

#[test]
fn spin_response_dto_jackpot_flag_propagates() {
    let mut spin = sample_spin();
    spin.is_jackpot = true;
    spin.payout = 100_000;
    spin.multiplier = 100.0;
    let result = SpinResult {
        spin,
        jackpot_pool_after: 1000,
        balance_after: 100_000,
        triggered_taunts: vec![],
    };
    let dto = SpinResponseDto::from(result);
    assert!(dto.is_jackpot);
    assert_eq!(dto.payout, 100_000);
    assert_eq!(dto.multiplier, 100.0);
}

#[test]
fn spin_response_dto_free_flag_propagates() {
    let mut spin = sample_spin();
    spin.is_free = true;
    let result = SpinResult {
        spin,
        jackpot_pool_after: 0,
        balance_after: 50,
        triggered_taunts: vec![],
    };
    let dto = SpinResponseDto::from(result);
    assert!(dto.is_free);
}

#[test]
fn top_winner_dto_from_entity() {
    let w = SlotTopWinner {
        user_id: "u1".into(),
        username: "Alice".into(),
        total_payout: 5000,
        jackpot_count: 2,
        spin_count: 25,
    };
    let dto = SlotTopWinnerDto::from(w);
    assert_eq!(dto.user_id, "u1".into());
    assert_eq!(dto.username, "Alice");
    assert_eq!(dto.total_payout, 5000);
    assert_eq!(dto.jackpot_count, 2);
    assert_eq!(dto.spin_count, 25);
}

#[test]
fn spin_response_serialization_to_json_works() {
    // Verifie que le DTO est bien Serializable (regression : avant, le champ
    // triggered_taunts plantait car TauntEvent n implementait pas Serialize ;
    // on passe maintenant par TauntEventDto).
    let result = SpinResult {
        spin: sample_spin(),
        jackpot_pool_after: 100,
        balance_after: 200,
        triggered_taunts: vec![],
    };
    let dto = SpinResponseDto::from(result);
    let json = serde_json::to_string(&dto).expect("DTO doit etre Serializable");
    assert!(json.contains("\"spin_id\""));
    assert!(json.contains("\"symbols\""));
    assert!(json.contains("\"jackpot_pool_after\":100"));
    assert!(json.contains("\"balance_after\":200"));
    assert!(json.contains("\"triggered_taunts\":[]"));
}
