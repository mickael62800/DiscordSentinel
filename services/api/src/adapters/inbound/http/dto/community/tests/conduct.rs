use super::*;
use crate::domain::entities::community::conduct::ConductConfig;
use crate::domain::entities::community::conduct::ConductPointsLog;
use crate::domain::entities::community::conduct::UserConductPoints;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn default_values_match_entity_defaults() {
    assert_eq!(default_max_points(), 12);
    assert_eq!(default_regen_amount(), 1);
    assert_eq!(default_regen_interval(), "weekly");
    assert_eq!(default_penalty_warn(), 1);
    assert_eq!(default_penalty_delete(), 2);
    assert_eq!(default_penalty_mute(), 3);
    assert_eq!(default_penalty_ban(), 6);
}

#[test]
fn penalty_defaults_form_increasing_gradient() {
    assert!(default_penalty_warn() < default_penalty_delete());
    assert!(default_penalty_delete() < default_penalty_mute());
    assert!(default_penalty_mute() < default_penalty_ban());
}

#[test]
fn save_config_dto_to_command() {
    let dto = SaveConductConfigDto {
        guild_id: "g".into(),
        max_points: 20,
        regen_amount: 2,
        regen_interval: "daily".into(),
        penalty_warn: 1,
        penalty_delete: 3,
        penalty_mute: 5,
        penalty_ban: 10,
    };
    let cmd: SaveConductConfigCommand = dto.into();
    assert_eq!(cmd.guild_id, "g");
    assert_eq!(cmd.max_points, 20);
    assert_eq!(cmd.regen_interval, "daily");
    assert_eq!(cmd.penalty_ban, 10);
}

#[test]
fn config_dto_from_entity_preserves_all_fields() {
    let c = ConductConfig::default_for_guild("g1");
    let dto: ConductConfigDto = c.into();
    assert_eq!(dto.guild_id, "g1");
    assert_eq!(dto.max_points, 12);
    assert_eq!(dto.penalty_ban, 6);
}

#[test]
fn user_points_dto_rfc3339_dates() {
    let now = Utc::now();
    let p = UserConductPoints {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "alice".into(),
        points: 8,
        last_regen_at: now,
        created_at: now,
        updated_at: now,
    };
    let dto: UserConductPointsDto = p.into();
    assert_eq!(dto.username, "alice");
    assert_eq!(dto.points, 8);
    assert!(dto.last_regen_at.contains('T')); // rfc3339 a un T
    assert!(dto.created_at.contains('T'));
}

#[test]
fn points_log_dto_preserves_delta_and_balance() {
    let log = ConductPointsLog {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        user_id: "u".into(),
        delta: -3,
        reason: "mute action".into(),
        points_before: 10,
        points_after: 7,
        created_at: Utc::now(),
    };
    let dto: ConductPointsLogDto = log.into();
    assert_eq!(dto.delta, -3);
    assert_eq!(dto.points_before, 10);
    assert_eq!(dto.points_after, 7);
    assert_eq!(dto.reason, "mute action");
    // Invariant : points_after = points_before + delta.
    assert_eq!(dto.points_before + dto.delta, dto.points_after);
}
