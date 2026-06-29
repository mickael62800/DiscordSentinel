//! Tests d'integration HTTP pour les endpoints audit_logs.

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
use sentinel_api::ports::inbound::audit::manage_audit_logs::AuditLogFilters;
use sentinel_api::ports::inbound::audit::manage_audit_logs::CreateAuditLogCommand;
use sentinel_api::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase;
use sentinel_core::domain::entities::audit::audit_log::AuditLog;
use sentinel_core::domain::errors::DomainError;
use test_helpers::build_test_state_audit_logs;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockAuditLogsUC {
    items: Mutex<Vec<AuditLog>>,
}

impl MockAuditLogsUC {
    fn new() -> Self {
        Self::default()
    }
    fn with(self, log: AuditLog) -> Self {
        self.items.lock().unwrap().push(log);
        self
    }
}

#[async_trait]
impl ManageAuditLogsUseCase for MockAuditLogsUC {
    async fn create(&self, cmd: CreateAuditLogCommand) -> Result<AuditLog, DomainError> {
        let log = AuditLog {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            event_type: cmd.event_type,
            actor_id: cmd.actor_id,
            actor_name: cmd.actor_name,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            channel_id: cmd.channel_id,
            channel_name: cmd.channel_name,
            details: cmd.details,
            created_at: Utc::now(),
        };
        self.items.lock().unwrap().push(log.clone());
        Ok(log)
    }
    async fn list(
        &self,
        guild_id: Option<&str>,
        filters: AuditLogFilters,
    ) -> Result<Vec<AuditLog>, DomainError> {
        let all = self.items.lock().unwrap();
        let matching: Vec<AuditLog> = all
            .iter()
            .filter(|l| guild_id.is_none_or(|g| l.guild_id == g))
            .filter(|l| {
                filters
                    .event_type
                    .as_deref()
                    .is_none_or(|e| l.event_type == e)
            })
            .filter(|l| {
                filters
                    .actor_id
                    .as_deref()
                    .is_none_or(|a| l.actor_id.as_deref() == Some(a))
            })
            .filter(|l| {
                filters
                    .target_id
                    .as_deref()
                    .is_none_or(|t| l.target_id.as_deref() == Some(t))
            })
            .skip(filters.offset as usize)
            .take(filters.limit as usize)
            .cloned()
            .collect();
        Ok(matching)
    }
    async fn delete_older_than_days(&self, guild_id: &str, _: i32) -> Result<u64, DomainError> {
        let mut items = self.items.lock().unwrap();
        let before = items.len();
        items.retain(|l| l.guild_id != guild_id);
        Ok((before - items.len()) as u64)
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_app(uc: MockAuditLogsUC) -> axum::Router {
    router::build_for_test(build_test_state_audit_logs(Arc::new(uc)))
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

async fn delete(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("DELETE")
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

fn sample_log(guild_id: &str, event_type: &str, actor: Option<&str>) -> AuditLog {
    AuditLog {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        event_type: event_type.into(),
        actor_id: actor.map(|s| s.into()),
        actor_name: actor.map(|_| "Admin".into()),
        target_id: None,
        target_name: None,
        channel_id: None,
        channel_name: None,
        details: serde_json::json!({}),
        created_at: Utc::now(),
    }
}

// ══════════════════════════════════════════════════════════
// POST /api/audit-logs
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_audit_log_minimal() {
    let app = build_app(MockAuditLogsUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "event_type": "config.update",
    });
    let (status, json) = post_json(app, "/api/audit-logs", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["event_type"], "config.update");
    assert_eq!(json["details"], serde_json::json!({}));
    assert!(json["id"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_audit_log_full_fields() {
    let app = build_app(MockAuditLogsUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "event_type": "role.grant",
        "actor_id": "a1",
        "actor_name": "Alice",
        "target_id": "t1",
        "target_name": "Bob",
        "channel_id": "c1",
        "channel_name": "general",
        "details": {"role": "admin"}
    });
    let (status, json) = post_json(app, "/api/audit-logs", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["actor_name"], "Alice");
    assert_eq!(json["target_name"], "Bob");
    assert_eq!(json["details"]["role"], "admin");
}

// ══════════════════════════════════════════════════════════
// GET /api/audit-logs
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_audit_logs_requires_guild_id() {
    let app = build_app(MockAuditLogsUC::new());
    let (status, json) = get(app, "/api/audit-logs").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("guild_id"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_audit_logs_scoped_to_guild() {
    let uc = MockAuditLogsUC::new()
        .with(sample_log("111111111111111111", "ban", Some("a1")))
        .with(sample_log("222222222222222222", "ban", Some("a1")));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/audit-logs?guild_id=111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_audit_logs_filter_by_event_type() {
    let uc = MockAuditLogsUC::new()
        .with(sample_log("111111111111111111", "ban", None))
        .with(sample_log("111111111111111111", "unban", None));
    let app = build_app(uc);
    let (status, json) = get(
        app,
        "/api/audit-logs?guild_id=111111111111111111&event_type=ban",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["event_type"], "ban");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_audit_logs_filter_by_actor_id() {
    let uc = MockAuditLogsUC::new()
        .with(sample_log("111111111111111111", "ban", Some("a1")))
        .with(sample_log("111111111111111111", "ban", Some("a2")));
    let app = build_app(uc);
    let (status, json) = get(
        app,
        "/api/audit-logs?guild_id=111111111111111111&actor_id=a1",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
}

// ══════════════════════════════════════════════════════════
// DELETE /api/audit-logs/{guild_id} (purge)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_audit_logs_returns_count() {
    let uc = MockAuditLogsUC::new()
        .with(sample_log("111111111111111111", "a", None))
        .with(sample_log("111111111111111111", "b", None));
    let app = build_app(uc);
    let (status, json) = delete(app, "/api/audit-logs/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["deleted"], 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_audit_logs_empty_returns_zero() {
    let app = build_app(MockAuditLogsUC::new());
    let (status, json) = delete(app, "/api/audit-logs/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["deleted"], 0);
}
