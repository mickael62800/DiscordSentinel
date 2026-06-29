use super::*;

// ── StealFailPenaltyDto ──

#[test]
fn steal_fail_penalty_dto_deserializes() {
    let raw = r#"{"thief_id":"t1","amount":250}"#;
    let dto: StealFailPenaltyDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.thief_id, "t1");
    assert_eq!(dto.amount, 250);
}

// ── Response DTOs serialize ──

#[test]
fn transfer_coins_response_serializes() {
    let r = TransferCoinsResponse {
        taunt_events: vec![],
    };
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"taunt_events\":[]"));
}

#[test]
fn steal_response_serializes_with_stolen_and_taunts() {
    let r = StealResponse {
        stolen: 500,
        taunt_events: vec![],
    };
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"stolen\":500"));
    assert!(json.contains("\"taunt_events\":[]"));
}

#[test]
fn steal_fail_penalty_response_serializes() {
    let r = StealFailPenaltyResponse {
        lost: 100,
        taunt_events: vec![],
    };
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"lost\":100"));
}
