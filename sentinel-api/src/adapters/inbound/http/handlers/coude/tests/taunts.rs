use super::*;

// ── TauntEventDto::from(TauntEvent) ──

#[test]
fn taunt_event_dto_from_domain() {
    let e = TauntEvent {
        channel_id: "c1".into(),
        target_user_id: "u1".into(),
        message: "mocking".into(),
        nickname_suffix: "le naze".into(),
        streak_kind: "bj_win_streak",
        streak_value: 5,
    };
    let dto = TauntEventDto::from(e);
    assert_eq!(dto.channel_id, "c1");
    assert_eq!(dto.target_user_id, "u1");
    assert_eq!(dto.message, "mocking");
    assert_eq!(dto.nickname_suffix, "le naze");
    assert_eq!(dto.streak_kind, "bj_win_streak");
    assert_eq!(dto.streak_value, 5);
}

#[test]
fn taunt_event_dto_with_empty_suffix() {
    let e = TauntEvent {
        channel_id: "c".into(),
        target_user_id: "u".into(),
        message: "".into(),
        nickname_suffix: "".into(),
        streak_kind: "win",
        streak_value: 0,
    };
    let dto = TauntEventDto::from(e);
    assert!(dto.nickname_suffix.is_empty());
    assert_eq!(dto.streak_value, 0);
}

#[test]
fn taunt_event_dto_serializes_to_json() {
    let dto = TauntEventDto {
        channel_id: "c".into(),
        target_user_id: "u".into(),
        message: "m".into(),
        nickname_suffix: "n".into(),
        streak_kind: "k".into(),
        streak_value: 3,
    };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"streak_kind\":\"k\""));
    assert!(json.contains("\"streak_value\":3"));
}

// ── MaybeTauntEventDto ──

#[test]
fn maybe_taunt_event_dto_none() {
    let dto = MaybeTauntEventDto { event: None };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"event\":null"));
}

#[test]
fn maybe_taunt_event_dto_some() {
    let dto = MaybeTauntEventDto {
        event: Some(TauntEventDto {
            channel_id: "c".into(),
            target_user_id: "u".into(),
            message: "m".into(),
            nickname_suffix: "n".into(),
            streak_kind: "win".into(),
            streak_value: 1,
        }),
    };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"event\":{"));
    assert!(json.contains("\"streak_kind\":\"win\""));
}

// ── EcoAmountDto ──

#[test]
fn eco_amount_dto_deserializes() {
    let dto: EcoAmountDto = serde_json::from_str(r#"{"amount":1500}"#).unwrap();
    assert_eq!(dto.amount, 1500);
}

// ── TauntsConfigDto ──

#[test]
fn taunts_config_dto_serializes_all_fields() {
    let dto = TauntsConfigDto {
        guild_id: "g".into(),
        channel_id: Some("c".into()),
        enabled: true,
        rename_enabled: false,
        messages_enabled: true,
        opt_outs: vec!["u1".into(), "u2".into()],
    };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"enabled\":true"));
    assert!(json.contains("\"rename_enabled\":false"));
    assert!(json.contains("\"opt_outs\":[\"u1\",\"u2\"]"));
}

#[test]
fn taunts_config_dto_none_channel_id() {
    let dto = TauntsConfigDto {
        guild_id: "g".into(),
        channel_id: None,
        enabled: false,
        rename_enabled: true,
        messages_enabled: true,
        opt_outs: vec![],
    };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"channel_id\":null"));
    assert!(json.contains("\"opt_outs\":[]"));
}

// ── UpdateTauntsConfigDto ──

#[test]
fn update_taunts_config_minimal_required_fields() {
    let raw = r#"{"channel_id":null,"enabled":false}"#;
    let dto: UpdateTauntsConfigDto = serde_json::from_str(raw).unwrap();
    assert!(!dto.enabled);
    assert!(dto.channel_id.is_none());
    assert!(dto.rename_enabled.is_none());
    assert!(dto.messages_enabled.is_none());
}

#[test]
fn update_taunts_config_full() {
    let raw = r#"{"channel_id":"c1","enabled":true,"rename_enabled":false,"messages_enabled":true}"#;
    let dto: UpdateTauntsConfigDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.channel_id.as_deref(), Some("c1"));
    assert!(dto.enabled);
    assert_eq!(dto.rename_enabled, Some(false));
    assert_eq!(dto.messages_enabled, Some(true));
}
