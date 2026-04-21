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
use axum::http::{Request, StatusCode};
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::domain::entities::SecurityEvent;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::{
    AnalyzeNewMemberCommand, ManageSecurityUseCase, ReportSecurityEventCommand, SecurityDecision,
};

use test_helpers::build_test_state_security;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockSecurityUC {
    events: Mutex<Vec<SecurityEvent>>,
}

impl MockSecurityUC {
    fn new() -> Self { Self::default() }
    fn with(self, e: SecurityEvent) -> Self {
        self.events.lock().unwrap().push(e);
        self
    }
}

#[async_trait]
impl ManageSecurityUseCase for MockSecurityUC {
    async fn report_event(&self, cmd: ReportSecurityEventCommand) -> Result<SecurityEvent, DomainError> {
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
    async fn list_events(&self, guild_id: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError> {
        let evs = self.events.lock().unwrap();
        let filtered: Vec<SecurityEvent> = evs.iter()
            .filter(|e| guild_id.is_none_or(|g| e.guild_id == g))
            .cloned()
            .collect();
        Ok(filtered)
    }
    async fn analyze_new_member(&self, _: AnalyzeNewMemberCommand) -> Result<SecurityDecision, DomainError> {
        Ok(SecurityDecision::default())
    }
}

fn build_app(uc: MockSecurityUC) -> axum::Router {
    router::build_for_test(build_test_state_security(Arc::new(uc)))
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

async fn post_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("POST").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
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
