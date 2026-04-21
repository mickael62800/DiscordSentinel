use super::*;
use crate::domain::entities::AuditLog;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn default_details_is_empty_object_when_missing() {
    let dto: CreateAuditLogDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "event_type": "x"
    }))
    .unwrap();
    assert_eq!(dto.details, serde_json::json!({}));
    assert!(dto.actor_id.is_none());
    assert!(dto.target_id.is_none());
}

#[test]
fn create_dto_to_command_preserves_all_fields() {
    let dto = CreateAuditLogDto {
        guild_id: "g".into(),
        event_type: "role.update".into(),
        actor_id: Some("a".into()),
        actor_name: Some("Admin".into()),
        target_id: Some("t".into()),
        target_name: Some("Target".into()),
        channel_id: Some("c".into()),
        channel_name: Some("general".into()),
        details: serde_json::json!({"from": "x"}),
    };
    let cmd: CreateAuditLogCommand = dto.into();
    assert_eq!(cmd.guild_id, "g");
    assert_eq!(cmd.event_type, "role.update");
    assert_eq!(cmd.actor_name.as_deref(), Some("Admin"));
    assert_eq!(cmd.details["from"], "x");
}

#[test]
fn from_audit_log_maps_all_fields() {
    let log = AuditLog {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        event_type: "ban".into(),
        actor_id: Some("a".into()),
        actor_name: None,
        target_id: None,
        target_name: Some("bob".into()),
        channel_id: None,
        channel_name: None,
        details: serde_json::json!({}),
        created_at: Utc::now(),
    };
    let id = log.id.to_string();
    let dto = AuditLogResponseDto::from(log);
    assert_eq!(dto.id, id);
    assert_eq!(dto.event_type, "ban");
    assert_eq!(dto.actor_id.as_deref(), Some("a"));
    assert!(dto.actor_name.is_none());
    assert_eq!(dto.target_name.as_deref(), Some("bob"));
    assert!(dto.created_at.contains('T'));
}

#[test]
fn query_params_all_optional() {
    let p: AuditLogQueryParams = serde_json::from_str("{}").unwrap();
    assert!(p.guild_id.is_none());
    assert!(p.event_type.is_none());
    assert!(p.actor_id.is_none());
    assert!(p.target_id.is_none());
    assert!(p.limit.is_none());
    assert!(p.offset.is_none());
}
