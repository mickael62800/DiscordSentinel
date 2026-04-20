use super::*;
use crate::domain::entities::{LevelConfig, LevelReward, UserLevel, XpSource};
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
    // xp_progress(200) = (200-155, 220) = (45, 220) selon level.rs test
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
    // xp_needed pour niveau 0+1 = 155 (voir tests level.rs)
    assert_eq!(dto.xp_needed, 155);
}

#[test]
fn user_level_dto_independent_sources() {
    let dto: UserLevelDto = user_level(1000, 500, 500).into();
    // Les sources text/voice sont calculees independamment.
    assert_ne!(dto.xp_current, dto.xp_text_current);
}

#[test]
fn level_reward_dto_source_as_str() {
    let r = LevelReward {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        level: 5,
        role_id: "role".into(),
        source: XpSource::Voice,
    };
    let dto: LevelRewardDto = r.into();
    assert_eq!(dto.source, "voice");
    assert_eq!(dto.level, 5);
}
