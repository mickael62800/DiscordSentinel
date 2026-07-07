use super::*;
use sentinel_core::domain::errors::DomainError;

#[test]
fn parse_role_accepts_all_four() {
    assert!(matches!(parse_role("owner").ok(), Some(Role::Owner)));
    assert!(matches!(parse_role("admin").ok(), Some(Role::Admin)));
    assert!(matches!(
        parse_role("moderator").ok(),
        Some(Role::Moderator)
    ));
    assert!(matches!(parse_role("viewer").ok(), Some(Role::Viewer)));
}

#[test]
fn parse_role_rejects_unknown_with_clear_message() {
    let err = match parse_role("superuser") {
        Ok(_) => panic!("expected err"),
        Err(e) => e,
    };
    match err.0 {
        DomainError::ValidationError(msg) => {
            assert!(msg.contains("superuser"));
            assert!(msg.contains("owner"));
        }
        other => panic!("expected ValidationError, got {other:?}"),
    }
}

#[test]
fn parse_role_rejects_empty() {
    assert!(parse_role("").is_err());
}

#[test]
fn status_to_err_forbidden_becomes_domain_forbidden() {
    let err = status_to_err(StatusCode::FORBIDDEN, "admin requis");
    match err.0 {
        DomainError::Forbidden(msg) => assert_eq!(msg, "admin requis"),
        other => panic!("expected Forbidden, got {other:?}"),
    }
}

#[test]
fn status_to_err_other_status_becomes_internal() {
    let err = status_to_err(StatusCode::UNAUTHORIZED, "ctx");
    match err.0 {
        DomainError::Internal(msg) => {
            assert!(msg.contains("401"));
            assert!(msg.contains("rbac gate"));
        }
        other => panic!("expected Internal, got {other:?}"),
    }
}

#[test]
fn status_to_err_service_unavailable_becomes_internal() {
    let err = status_to_err(StatusCode::SERVICE_UNAVAILABLE, "ctx");
    match err.0 {
        DomainError::Internal(_) => {}
        other => panic!("expected Internal, got {other:?}"),
    }
}

// ── DTO tests ────────────────────────────────────────────────

#[test]
fn grant_role_dto_default_display_name_none() {
    let dto: GrantRoleDto = serde_json::from_value(serde_json::json!({
        "role": "admin"
    }))
    .unwrap();
    assert_eq!(dto.role, "admin");
    assert!(dto.display_name.is_none());
}

#[test]
fn grant_role_dto_with_display_name() {
    let dto: GrantRoleDto = serde_json::from_value(serde_json::json!({
        "role": "owner", "display_name": "Alice"
    }))
    .unwrap();
    assert_eq!(dto.display_name.as_deref(), Some("Alice"));
}

#[test]
fn update_role_dto_deserializes() {
    let dto: UpdateRoleDto = serde_json::from_value(serde_json::json!({"role": "viewer"})).unwrap();
    assert_eq!(dto.role, "viewer");
}
