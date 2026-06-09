use super::*;
use sentinel_core::domain::entities::community::level::LevelConfig;
use sentinel_core::domain::entities::community::level::UserLevel;
use sentinel_core::domain::entities::community::level::XpSource;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn default_values() {
    assert_eq!(default_xp_per_message(), 15);
    assert_eq!(default_xp_per_voice_minute(), 5);
    assert_eq!(default_xp_cooldown(), 60);
    assert!(default_enabled());
    assert_eq!(default_source(), "text");
    assert!(default_level_up_message().contains("{user}"));
    assert!(default_level_up_message().contains("{level}"));
}

#[test]
fn save_config_dto_to_command() {
    let dto = SaveLevelConfigDto {
        guild_id: "g".into(),
        xp_per_message: 20,
        xp_per_voice_minute: 10,
        xp_cooldown_secs: 120,
        level_up_channel_id: Some("chan".into()),
        level_up_message: "hey".into(),
        excluded_channels: vec!["c1".into(), "c2".into()],
        enabled: false,
    };
    let cmd: SaveLevelConfigCommand = dto.into();
    assert_eq!(cmd.xp_per_message, 20);
    assert_eq!(cmd.excluded_channels.len(), 2);
    assert_eq!(cmd.level_up_channel_id.unwrap(), "chan");
    assert!(!cmd.enabled);
}

#[test]
fn level_config_dto_from_entity() {
    let now = Utc::now();
    let c = LevelConfig {
        guild_id: "g".into(),
        xp_per_message: 15,
        xp_per_voice_minute: 5,
        xp_cooldown_secs: 60,
        level_up_channel_id: None,
        level_up_message: "m".into(),
        excluded_channels: vec![],
        enabled: true,
        created_at: now,
        updated_at: now,
    };
    let dto: LevelConfigDto = c.into();
    assert_eq!(dto.xp_per_message, 15);
    assert!(dto.enabled);
}

fn user_level(xp: i64, xp_text: i64, xp_voice: i64) -> UserLevel {
    let now = Utc::now();
    UserLevel {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "alice".into(),
        xp,
        level: 0,
        xp_text,
        level_text: 0,
        xp_voice,
        level_voice: 0,
        last_xp_at: now,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn user_level_dto_computes_xp_progress_for_each_source() {
    let dto: UserLevelDto = user_level(200, 200, 200).into();
    assert_eq!(dto.xp_current, 45);
    assert_eq!(dto.xp_needed, 220);
    assert_eq!(dto.xp_text_current, 45);
    assert_eq!(dto.xp_voice_current, 45);
}

#[test]
fn user_level_dto_zero_xp_returns_zero_progress() {
    let dto: UserLevelDto = user_level(0, 0, 0).into();
    assert_eq!(dto.xp_current, 0);
    assert_eq!(dto.xp_needed, 155);
}

#[test]
fn user_level_dto_independent_sources() {
    let dto: UserLevelDto = user_level(1000, 500, 500).into();
    assert_ne!(dto.xp_current, dto.xp_text_current);
}

use crate::ports::inbound::community::manage_levels::AddXpResult;

#[test]
fn save_config_dto_deserializes_with_defaults() {
    let dto: SaveLevelConfigDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g"
    })).unwrap();
    assert_eq!(dto.xp_per_message, 15);
    assert_eq!(dto.xp_per_voice_minute, 5);
    assert_eq!(dto.xp_cooldown_secs, 60);
    assert!(dto.enabled);
    assert!(dto.excluded_channels.is_empty());
    assert!(dto.level_up_message.contains("{user}"));
}

#[test]
fn save_config_dto_deserializes_with_overrides() {
    let dto: SaveLevelConfigDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g",
        "xp_per_message": 25,
        "xp_per_voice_minute": 8,
        "xp_cooldown_secs": 30,
        "level_up_channel_id": "c1",
        "excluded_channels": ["ex1", "ex2"],
        "enabled": false
    })).unwrap();
    assert_eq!(dto.xp_per_message, 25);
    assert_eq!(dto.level_up_channel_id.as_deref(), Some("c1"));
    assert_eq!(dto.excluded_channels.len(), 2);
    assert!(!dto.enabled);
}

#[test]
fn add_xp_dto_default_source_is_text() {
    let dto: AddXpDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "user_id": "u", "username": "alice", "amount": 100
    })).unwrap();
    assert_eq!(dto.source, "text");
}

#[test]
fn add_xp_dto_with_voice_source() {
    let dto: AddXpDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "user_id": "u", "username": "alice",
        "amount": 50, "source": "voice"
    })).unwrap();
    assert_eq!(dto.source, "voice");
    assert_eq!(dto.amount, 50);
}

#[test]
fn level_leaderboard_params_all_optional() {
    let p: LevelLeaderboardParams = serde_json::from_str("{}").unwrap();
    assert!(p.limit.is_none());
    assert!(p.source.is_none());
}

#[test]
fn level_leaderboard_params_with_source() {
    let p: LevelLeaderboardParams = serde_json::from_value(serde_json::json!({
        "limit": 20, "source": "voice"
    })).unwrap();
    assert_eq!(p.limit, Some(20));
    assert_eq!(p.source.as_deref(), Some("voice"));
}

#[test]
fn add_xp_response_dto_from_result_with_level_up() {
    let now = Utc::now();
    let result = AddXpResult {
        user_level: UserLevel {
            id: Uuid::new_v4(), guild_id: "g".into(), user_id: "u".into(),
            username: "alice".into(),
            xp: 500, level: 3,
            xp_text: 400, level_text: 2,
            xp_voice: 100, level_voice: 1,
            last_xp_at: now, created_at: now, updated_at: now,
        },
        leveled_up: true,
        old_level: 2,
        old_level_global: 2,
        source: XpSource::Text,
    };
    let dto: AddXpResponseDto = result.into();
    assert!(dto.leveled_up);
    assert_eq!(dto.old_level, 2);
    assert_eq!(dto.source, "text");
    assert_eq!(dto.user.xp, 500);
}

#[test]
fn add_xp_response_dto_no_level_up() {
    let now = Utc::now();
    let result = AddXpResult {
        user_level: UserLevel {
            id: Uuid::new_v4(), guild_id: "g".into(), user_id: "u".into(),
            username: "alice".into(),
            xp: 50, level: 0, xp_text: 50, level_text: 0,
            xp_voice: 0, level_voice: 0,
            last_xp_at: now, created_at: now, updated_at: now,
        },
        leveled_up: false,
        old_level: 0,
        old_level_global: 0,
        source: XpSource::Voice,
    };
    let dto: AddXpResponseDto = result.into();
    assert!(!dto.leveled_up);
    assert_eq!(dto.source, "voice");
}
