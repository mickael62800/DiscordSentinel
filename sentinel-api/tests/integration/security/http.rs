//! Tests d'integration HTTP pour les endpoints security.
//!
//! Note : le handler `purge_events` fait du sqlx direct et n'est pas
//! testable sans DB — couvert par les tests d'integration db-backed.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::ports::inbound::audit::manage_security::AnalyzeNewMemberCommand;
use sentinel_api::ports::inbound::audit::manage_security::ManageSecurityUseCase;
use sentinel_api::ports::inbound::audit::manage_security::ReportSecurityEventCommand;
use sentinel_api::ports::inbound::audit::manage_security::SecurityDecision;
use sentinel_core::domain::entities::audit::security_event::SecurityEvent;
use sentinel_core::domain::errors::DomainError;
use test_helpers::build_test_state_security;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockSecurityUC {
    events: Mutex<Vec<SecurityEvent>>,
}

impl MockSecurityUC {
    fn new() -> Self {
        Self::default()
    }
    fn with(self, e: SecurityEvent) -> Self {
        self.events.lock().unwrap().push(e);
        self
    }
}

#[async_trait]
impl ManageSecurityUseCase for MockSecurityUC {
    async fn report_event(
        &self,
        cmd: ReportSecurityEventCommand,
    ) -> Result<SecurityEvent, DomainError> {
        let event = SecurityEvent {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            event_type: cmd.event_type,
            severity: cmd.severity,
            description: cmd.description,
            user_ids: cmd.user_ids,
            created_at: Utc::now(),
        };
        self.events.lock().unwrap().push(event.clone());
        Ok(event)
    }
    async fn purge_events(&self, _: &str) -> Result<(u64, u64), DomainError> {
        Ok((0, 0))
    }
    async fn list_events(&self, guild_id: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError> {
        let evs = self.events.lock().unwrap();
        let filtered: Vec<SecurityEvent> = evs
            .iter()
            .filter(|e| guild_id.is_none_or(|g| e.guild_id.as_str() == g))
            .cloned()
            .collect();
        Ok(filtered)
    }
    async fn analyze_new_member(
        &self,
        _: AnalyzeNewMemberCommand,
    ) -> Result<SecurityDecision, DomainError> {
        Ok(SecurityDecision::default())
    }
}

fn build_app(uc: MockSecurityUC) -> axum::Router {
    router::build_for_test(build_test_state_security(Arc::new(uc)))
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

async fn post_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

fn sample_event(guild_id: &str, severity: &str) -> SecurityEvent {
    SecurityEvent {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        event_type: "raid_detected".into(),
        severity: severity.into(),
        description: "mass join".into(),
        user_ids: vec!["u1".into(), "u2".into()],
        created_at: Utc::now(),
    }
}

// ══════════════════════════════════════════════════════════
// POST /api/security/events
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_event_success() {
    let app = build_app(MockSecurityUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "event_type": "raid_detected",
        "severity": "high",
        "description": "Raid detecte",
        "user_ids": ["u1", "u2"]
    });
    let (status, json) = post_json(app, "/api/security/events", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["event_type"], "raid_detected");
    assert_eq!(json["severity"], "high");
    assert_eq!(json["user_ids"].as_array().unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_event_defaults_user_ids_empty() {
    let app = build_app(MockSecurityUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "event_type": "suspicious_activity",
        "severity": "low",
        "description": "desc"
    });
    let (status, json) = post_json(app, "/api/security/events", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["user_ids"].as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_event_invalid_guild_id_422() {
    let app = build_app(MockSecurityUC::new());
    let body = serde_json::json!({
        "guild_id": "not-a-snowflake",
        "event_type": "x", "severity": "y", "description": "z"
    });
    let (status, _) = post_json(app, "/api/security/events", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_event_description_too_long_422() {
    let app = build_app(MockSecurityUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "event_type": "x", "severity": "y",
        "description": "a".repeat(3000)
    });
    let (status, _) = post_json(app, "/api/security/events", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// GET /api/security/events
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_events_returns_all_without_filter() {
    let uc = MockSecurityUC::new()
        .with(sample_event("111111111111111111", "high"))
        .with(sample_event("222222222222222222", "low"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/security/events").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_events_filter_by_guild() {
    let uc = MockSecurityUC::new()
        .with(sample_event("111111111111111111", "high"))
        .with(sample_event("222222222222222222", "low"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/security/events?guild_id=111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["severity"], "high");
}

// ══════════════════════════════════════════════════════════
// purge_events (sqlx direct -> utilise la vraie DB test)
// ══════════════════════════════════════════════════════════

async fn pool() -> sqlx::PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    sqlx::PgPool::connect(&url).await.unwrap()
}

async fn insert_audit_security_event(pool: &sqlx::PgPool, guild_id: &str, event_type: &str) {
    sqlx::query(
        "INSERT INTO audit_logs (id, guild_id, event_type, details, created_at) \
         VALUES ($1, $2, $3, '{}'::jsonb, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(guild_id)
    .bind(event_type)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_manual_watch(pool: &sqlx::PgPool, guild_id: &str, user_id: &str, added_by: &str) {
    sqlx::query(
        "INSERT INTO manual_watched_users (guild_id, user_id, username, added_by) \
         VALUES ($1, $2, 'User', $3)",
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(added_by)
    .execute(pool)
    .await
    .unwrap();
}

async fn delete_req(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_events_removes_audit_and_manual_watches() {
    let p = pool().await;
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let user_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );

    // Audit logs security_* : 2 a purger
    insert_audit_security_event(&p, &guild_id, "security_raid").await;
    insert_audit_security_event(&p, &guild_id, "security_alt").await;
    // Audit logs non security : doit survivre
    insert_audit_security_event(&p, &guild_id, "config_update").await;

    // manual_watched_users cree par security_event : a purger
    insert_manual_watch(&p, &guild_id, &user_id, "security_event").await;
    // manual_watched_users manuels : survivent
    let other_user = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    insert_manual_watch(&p, &guild_id, &other_user, "desktop").await;

    let app = build_app(MockSecurityUC::new());
    let (status, json) = delete_req(app, &format!("/api/security/events/{guild_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["deleted_events"], 2);
    assert_eq!(json["deleted_watches"], 1);

    // Audit config_update doit rester
    let remaining_audit = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1 AND event_type = 'config_update'",
    )
    .bind(&guild_id)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(remaining_audit, 1);
    // manual_watch desktop doit rester
    let remaining_watch = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM manual_watched_users WHERE guild_id = $1 AND added_by = 'desktop'",
    )
    .bind(&guild_id)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(remaining_watch, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_events_empty_returns_zero() {
    let app = build_app(MockSecurityUC::new());
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let (status, json) = delete_req(app, &format!("/api/security/events/{guild_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["deleted_events"], 0);
    assert_eq!(json["deleted_watches"], 0);
}

// RBAC : viewer forbidden, admin allowed via injection manuelle

async fn send_request(
    app: axum::Router,
    req: axum::http::Request<Body>,
) -> (StatusCode, serde_json::Value) {
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_events_with_rbac_moderator_succeeds() {
    use sentinel_core::domain::enums::system::role::Role;
    let p = pool().await;
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let user_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    sqlx::query("INSERT INTO api_users (discord_user_id, display_name) VALUES ($1, 'M') ON CONFLICT DO NOTHING")
        .bind(&user_id).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO api_user_guilds (discord_user_id, guild_id, role) VALUES ($1, $2, 'moderator')")
        .bind(&user_id).bind(&guild_id).execute(&p).await.unwrap();

    let app = build_app(MockSecurityUC::new());
    let req = test_helpers::request_with_rbac(
        "DELETE",
        &format!("/api/security/events/{guild_id}"),
        &user_id,
        Some(Role::Moderator),
        Some(guild_id.clone()),
        None,
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_events_with_rbac_viewer_forbidden() {
    use sentinel_core::domain::enums::system::role::Role;
    let p = pool().await;
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let user_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    sqlx::query("INSERT INTO api_users (discord_user_id, display_name) VALUES ($1, 'V') ON CONFLICT DO NOTHING")
        .bind(&user_id).execute(&p).await.unwrap();
    sqlx::query(
        "INSERT INTO api_user_guilds (discord_user_id, guild_id, role) VALUES ($1, $2, 'viewer')",
    )
    .bind(&user_id)
    .bind(&guild_id)
    .execute(&p)
    .await
    .unwrap();

    let app = build_app(MockSecurityUC::new());
    let req = test_helpers::request_with_rbac(
        "DELETE",
        &format!("/api/security/events/{guild_id}"),
        &user_id,
        Some(Role::Viewer),
        Some(guild_id.clone()),
        None,
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
