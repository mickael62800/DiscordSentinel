use super::*;

#[test]
fn credit_debit_dto_deserializes() {
    let raw = r#"{"amount":500,"source":"coude_combat_win","description":"Victoire"}"#;
    let dto: CreditDebitDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.amount, 500);
    assert_eq!(dto.source, "coude_combat_win");
    assert_eq!(dto.description, "Victoire");
}

#[test]
fn transfer_dto_deserializes() {
    let raw = r#"{"guild_id":"g","from_user_id":"u1","to_user_id":"u2","amount":100,"source":"don","description":"cadeau"}"#;
    let dto: TransferDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.guild_id, "g".into());
    assert_eq!(dto.from_user_id, "u1");
    assert_eq!(dto.to_user_id, "u2");
    assert_eq!(dto.amount, 100);
}

#[test]
fn limit_query_none_default() {
    let q: LimitQuery = serde_json::from_str("{}").unwrap();
    assert!(q.limit.is_none());
}

#[test]
fn limit_query_with_value() {
    let q: LimitQuery = serde_json::from_str(r#"{"limit":50}"#).unwrap();
    assert_eq!(q.limit, Some(50));
}

#[test]
fn reset_wallet_dto_without_balance_is_none() {
    let dto: ResetWalletDto = serde_json::from_str("{}").unwrap();
    assert!(dto.new_balance.is_none());
}

#[test]
fn reset_wallet_dto_with_balance() {
    let dto: ResetWalletDto = serde_json::from_str(r#"{"new_balance":1000}"#).unwrap();
    assert_eq!(dto.new_balance, Some(1000));
}
