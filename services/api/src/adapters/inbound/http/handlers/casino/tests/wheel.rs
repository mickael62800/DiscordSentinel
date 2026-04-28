//! Tests des conversions DTO du handler wheel.

use chrono::Utc;
use uuid::Uuid;

use crate::adapters::inbound::http::handlers::wheel::{
    WheelSpinLogDto, WheelSpinResponseDto, WheelTopWinnerDto,
};
use crate::domain::entities::{WheelCase, WheelSpin, WheelTopWinner};
use crate::ports::inbound::manage_wheel::WheelSpinResult;

fn sample_jackpot_case() -> WheelCase {
    WheelCase { key: "jackpot", label: "🎰 Jackpot", payout: 5000, weight: 3 }
}

fn sample_spin() -> WheelSpin {
    WheelSpin {
        id: Uuid::nil(),
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "Alice".into(),
        case_key: "jackpot".into(),
        case_label: "🎰 Jackpot".into(),
        payout: 5000,
        created_at: Utc::now(),
    }
}

#[test]
fn spin_log_dto_from_entity() {
    let s = sample_spin();
    let dto = WheelSpinLogDto::from(s.clone());
    assert_eq!(dto.user_id, "u");
    assert_eq!(dto.username, "Alice");
    assert_eq!(dto.case_key, "jackpot");
    assert_eq!(dto.payout, 5000);
}

#[test]
fn spin_response_dto_from_result_jackpot() {
    let result = WheelSpinResult {
        spin: sample_spin(),
        case: sample_jackpot_case(),
        balance_after: 6000,
        is_memorable: true,
        triggered_taunts: vec![],
    };
    let dto = WheelSpinResponseDto::from(result);
    assert_eq!(dto.case_key, "jackpot");
    assert_eq!(dto.payout, 5000);
    assert_eq!(dto.balance_after, 6000);
    assert!(dto.is_memorable);
    assert_eq!(dto.triggered_taunts.len(), 0);
}

#[test]
fn spin_response_dto_negative_payout() {
    let result = WheelSpinResult {
        spin: WheelSpin {
            payout: -500,
            case_key: "ruine".into(),
            case_label: "💀 Ruine".into(),
            ..sample_spin()
        },
        case: WheelCase { key: "ruine", label: "💀 Ruine", payout: -500, weight: 5 },
        balance_after: 100,
        is_memorable: false,
        triggered_taunts: vec![],
    };
    let dto = WheelSpinResponseDto::from(result);
    assert_eq!(dto.payout, -500);
    assert!(!dto.is_memorable);
    assert!(dto.case_label.contains("Ruine"));
}

#[test]
fn top_winner_dto_from_entity() {
    let w = WheelTopWinner {
        user_id: "u1".into(),
        username: "Bob".into(),
        total_payout: 12500,
        spin_count: 7,
    };
    let dto = WheelTopWinnerDto::from(w);
    assert_eq!(dto.username, "Bob");
    assert_eq!(dto.total_payout, 12500);
    assert_eq!(dto.spin_count, 7);
}

#[test]
fn spin_response_serializes_to_json() {
    // Regression : verifie que le DTO est Serializable (pas de TauntEvent
    // brut, mais TauntEventDto).
    let result = WheelSpinResult {
        spin: sample_spin(),
        case: sample_jackpot_case(),
        balance_after: 6000,
        is_memorable: true,
        triggered_taunts: vec![],
    };
    let dto = WheelSpinResponseDto::from(result);
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"case_key\":\"jackpot\""));
    assert!(json.contains("\"payout\":5000"));
    assert!(json.contains("\"is_memorable\":true"));
    assert!(json.contains("\"triggered_taunts\":[]"));
}
