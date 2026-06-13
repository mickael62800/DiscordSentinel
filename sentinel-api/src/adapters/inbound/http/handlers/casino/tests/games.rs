use super::*;

// ── CreateGameDto ──

#[test]
fn create_game_dto_minimal() {
    let raw = r#"{"guild_id":"g","game_name":"Valorant","created_by":"u1"}"#;
    let dto: CreateGameDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.game_name, "Valorant");
    assert!(dto.emoji.is_none());
    assert!(dto.category.is_none());
}

#[test]
fn create_game_dto_with_emoji_and_category() {
    let raw = r#"{"guild_id":"g","game_name":"LoL","created_by":"u","emoji":"🎮","category":"MOBA"}"#;
    let dto: CreateGameDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.emoji.as_deref(), Some("🎮"));
    assert_eq!(dto.category.as_deref(), Some("MOBA"));
}

// ── SetRoleIdDto ──

#[test]
fn set_role_id_dto_null_role_id() {
    let dto: SetRoleIdDto = serde_json::from_str(r#"{"role_id":null}"#).unwrap();
    assert!(dto.role_id.is_none());
}

#[test]
fn set_role_id_dto_absent_is_none() {
    let dto: SetRoleIdDto = serde_json::from_str(r#"{}"#).unwrap();
    assert!(dto.role_id.is_none());
}

#[test]
fn set_role_id_dto_with_value() {
    let dto: SetRoleIdDto = serde_json::from_str(r#"{"role_id":"1234"}"#).unwrap();
    assert_eq!(dto.role_id.as_deref(), Some("1234"));
}

// ── SavePanelDto ──

#[test]
fn save_panel_dto_minimal() {
    let raw = r#"{"channel_id":"c","message_id":"m"}"#;
    let dto: SavePanelDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.channel_id, "c".into());
    assert!(dto.category.is_none());
}

#[test]
fn save_panel_dto_full() {
    let raw = r#"{"channel_id":"c","message_id":"m","category":"FPS"}"#;
    let dto: SavePanelDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.category.as_deref(), Some("FPS"));
}

// ── UpdateGameDto (Option<Option<String>> tri-state) ──

#[test]
fn update_game_dto_all_none_absent() {
    let dto: UpdateGameDto = serde_json::from_str(r#"{}"#).unwrap();
    assert!(dto.game_name.is_none());
    assert!(dto.emoji.is_none());
    assert!(dto.category.is_none());
}

#[test]
fn update_game_dto_explicit_null_emoji() {
    // emoji: null → Some(None) (reset)
    let dto: UpdateGameDto = serde_json::from_str(r#"{"emoji":null}"#).unwrap();
    assert_eq!(dto.emoji, Some(None));
}

#[test]
fn update_game_dto_emoji_with_value() {
    let dto: UpdateGameDto = serde_json::from_str(r#"{"emoji":"🎯"}"#).unwrap();
    assert_eq!(dto.emoji, Some(Some("🎯".into())));
}

// ── CategoryQuery ──

#[test]
fn category_query_empty_is_none() {
    let q: CategoryQuery = serde_json::from_str(r#"{}"#).unwrap();
    assert!(q.category.is_none());
}

#[test]
fn category_query_with_value() {
    let q: CategoryQuery = serde_json::from_str(r#"{"category":"MOBA"}"#).unwrap();
    assert_eq!(q.category.as_deref(), Some("MOBA"));
}

// ── Serialize response DTOs ──

#[test]
fn game_panel_dto_serializes() {
    let dto = GamePanelDto {
        id: "a".into(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        message_id: "m".into(),
        category: Some("RPG".into()),
    };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"channel_id\":\"c\""));
    assert!(json.contains("\"category\":\"RPG\""));
}

#[test]
fn upload_emoji_response_serializes() {
    let r = UploadEmojiResponse {
        emoji: "<:test:123>".into(),
        emoji_id: "123".into(),
        name: "test".into(),
        animated: false,
    };
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"name\":\"test\""));
    assert!(json.contains("\"animated\":false"));
}
