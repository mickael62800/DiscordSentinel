use super::*;
use crate::domain::entities::{ModerationAction, UserModerationHistory};
use crate::domain::value_objects::ModerationGravity;
use chrono::Utc;
use uuid::Uuid;

fn sample_action() -> ModerationAction {
    ModerationAction {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod".into(),
        moderator_name: "Mod".into(),
        target_id: "u".into(),
        target_name: "Alice".into(),
        action_type: "ban_temp".into(),
        reason: "spam".into(),
        gravity: Some(ModerationGravity::High),
        duration: Some(3600),
        created_at: Utc::now(),
    }
}

#[test]
fn log_action_dto_to_command_preserves_fields() {
    let dto = LogActionDto {
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod".into(),
        moderator_name: "Mod".into(),
        target_id: "u".into(),
        target_name: "Alice".into(),
        action_type: "warn".into(),
        reason: "test".into(),
        gravity: Some("medium".into()),
        duration: Some(600),
    };
    let cmd: LogModerationCommand = dto.into();
    assert_eq!(cmd.action_type, "warn");
    assert_eq!(cmd.gravity, Some("medium".into()));
    assert_eq!(cmd.duration, Some(600));
}

#[test]
fn log_action_dto_optional_fields_none() {
    let dto = LogActionDto {
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod".into(),
        moderator_name: "M".into(),
        target_id: "u".into(),
        target_name: "U".into(),
        action_type: "warn".into(),
        reason: "x".into(),
        gravity: None,
        duration: None,
    };
    let cmd: LogModerationCommand = dto.into();
    assert!(cmd.gravity.is_none());
    assert!(cmd.duration.is_none());
}

#[test]
fn action_to_response_dto_strips_metadata() {
    let a = sample_action();
    let dto: ModerationActionResponseDto = a.into();
    // Metadata d'escalation est None par defaut (pas mappee directement).
    assert!(dto.escalation_action.is_none());
    assert!(dto.escalation_duration.is_none());
    assert!(dto.strikes_count.is_none());
    assert_eq!(dto.action_type, "ban_temp");
    assert_eq!(dto.target_name, "Alice");
}

#[test]
fn action_to_ban_entry_dto_copies_ids() {
    let a = sample_action();
    let id = a.id;
    let dto: BanEntryDto = a.into();
    assert_eq!(dto.id, id.to_string());
    assert_eq!(dto.action_type, "ban_temp");
    assert!(dto.created_at.contains('T'));
}

#[test]
fn user_history_to_dto_aggregates() {
    let history = UserModerationHistory {
        target_id: "u".into(),
        target_name: "Alice".into(),
        total_warns: 3,
        total_mutes: 1,
        total_bans: 0,
        actions: vec![sample_action(), sample_action()],
    };
    let dto: UserHistoryDto = history.into();
    assert_eq!(dto.total_warns, 3);
    assert_eq!(dto.actions.len(), 2);
}
