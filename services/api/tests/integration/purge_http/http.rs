//! Tests d'integration HTTP pour les endpoints purge.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::adapters::inbound::http::state::AppState;
use sentinel_api::domain::entities::{AuditLog, LogEntry};
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::{
    InfractionFilters, ManageAuditLogsUseCase, ManageInfractionsUseCase,
};
use sentinel_api::ports::inbound::manage_audit_logs::{AuditLogFilters, CreateAuditLogCommand};
use sentinel_api::domain::entities::Infraction;
use sentinel_api::ports::outbound::LogRepository;

#[derive(Default)]
struct MockInfUC {
    purged: Mutex<Vec<(String, i32)>>,
}
#[async_trait]
impl ManageInfractionsUseCase for MockInfUC {
    async fn list_infractions(&self, _: &str, _: InfractionFilters) -> Result<Vec<Infraction>, DomainError> { Ok(vec![]) }
    async fn list_all_infractions(&self, _: i64, _: i64) -> Result<Vec<Infraction>, DomainError> { Ok(vec![]) }
    async fn count_today(&self) -> Result<u64, DomainError> { Ok(0) }
    async fn find_by_id(&self, _: &str) -> Result<Option<Infraction>, DomainError> { Ok(None) }
    async fn delete_infraction(&self, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError> {
        self.purged.lock().unwrap().push((guild_id.into(), days));
        Ok(42)
    }
}

#[derive(Default)]
struct MockAuditUC {
    purged: Mutex<Vec<(String, i32)>>,
}
#[async_trait]
impl ManageAuditLogsUseCase for MockAuditUC {
    async fn create(&self, _: CreateAuditLogCommand) -> Result<AuditLog, DomainError> { unimplemented!() }
    async fn list(&self, _: Option<&str>, _: AuditLogFilters) -> Result<Vec<AuditLog>, DomainError> { Ok(vec![]) }
    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError> {
        self.purged.lock().unwrap().push((guild_id.into(), days));
        Ok(7)
    }
}

#[derive(Default)]
struct MockLogRepo {
    purged: Mutex<Vec<i32>>,
}
#[async_trait]
impl LogRepository for MockLogRepo {
    async fn save(&self, _: &LogEntry) -> Result<(), DomainError> { Ok(()) }
    async fn find_all(&self, _: i64) -> Result<Vec<LogEntry>, DomainError> { Ok(vec![]) }
    async fn delete_by_category(&self, _: &str) -> Result<u64, DomainError> { Ok(0) }
    async fn delete_older_than_days(&self, days: i32) -> Result<u64, DomainError> {
        self.purged.lock().unwrap().push(days);
        Ok(99)
    }
}

fn build_state() -> (AppState, Arc<MockInfUC>, Arc<MockAuditUC>, Arc<MockLogRepo>) {
    let inf = Arc::new(MockInfUC::default());
    let audit = Arc::new(MockAuditUC::default());
    let log = Arc::new(MockLogRepo::default());
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.infractions_uc = inf.clone();
    state.audit_logs_uc = audit.clone();
    state.log_repo = log.clone();
    (state, inf, audit, log)
}

async fn delete_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("DELETE").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null))
}

async fn send_request(app: axum::Router, req: Request<Body>) -> (StatusCode, serde_json::Value) {
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null))
}

// ══════════════════════════════════════════════════════════
// purge_infractions
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_infractions_success_positive_days() {
    let (state, inf, _, _) = build_state();
    let app = router::build_for_test(state);
    let body = serde_json::json!({ "guild_id": "111111111111111111", "days": 30 });
    let (status, json) = delete_json(app, "/api/purge/infractions", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["deleted"], 42);
    assert_eq!(inf.purged.lock().unwrap()[0], ("111111111111111111".into(), 30));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_infractions_allows_zero_meaning_all() {
    let (state, inf, _, _) = build_state();
    let app = router::build_for_test(state);
    let body = serde_json::json!({ "guild_id": "111111111111111111", "days": 0 });
    let (status, _) = delete_json(app, "/api/purge/infractions", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(inf.purged.lock().unwrap()[0].1, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_infractions_rejects_negative_days() {
    let (state, _, _, _) = build_state();
    let app = router::build_for_test(state);
    let body = serde_json::json!({ "guild_id": "111111111111111111", "days": -1 });
    let (status, json) = delete_json(app, "/api/purge/infractions", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains(">= 0"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_infractions_invalid_guild_422() {
    let (state, _, _, _) = build_state();
    let app = router::build_for_test(state);
    let body = serde_json::json!({ "guild_id": "bad", "days": 30 });
    let (status, _) = delete_json(app, "/api/purge/infractions", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_infractions_with_rbac_moderator_forbidden() {
    use sentinel_api::adapters::inbound::http::middleware::rbac::Role;
    let (state, _, _, _) = build_state();
    let app = router::build_for_test(state);
    let req = test_helpers::request_with_rbac(
        "DELETE", "/api/purge/infractions",
        "444444444444444444", Some(Role::Moderator),
        Some("111111111111111111".into()),
        Some(serde_json::json!({"guild_id": "111111111111111111", "days": 30})),
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_infractions_with_rbac_owner_allowed() {
    use sentinel_api::adapters::inbound::http::middleware::rbac::Role;
    // Seed api_user_guilds pour que check_role_for_guild valide owner en DB
    let pool = sqlx::PgPool::connect(
        &std::env::var("DATABASE_URL").unwrap_or_else(|_|
            "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into())
    ).await.unwrap();
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let user_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    sqlx::query("INSERT INTO api_users (discord_user_id, display_name) VALUES ($1, 'O') ON CONFLICT DO NOTHING")
        .bind(&user_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO api_user_guilds (discord_user_id, guild_id, role) VALUES ($1, $2, 'owner')")
        .bind(&user_id).bind(&guild_id).execute(&pool).await.unwrap();

    let (state, _, _, _) = build_state();
    let app = router::build_for_test(state);
    let req = test_helpers::request_with_rbac(
        "DELETE", "/api/purge/infractions",
        &user_id, Some(Role::Owner), Some(guild_id.clone()),
        Some(serde_json::json!({"guild_id": guild_id, "days": 30})),
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
}

// ══════════════════════════════════════════════════════════
// purge_audit_logs
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_audit_logs_success() {
    let (state, _, audit, _) = build_state();
    let app = router::build_for_test(state);
    let body = serde_json::json!({ "guild_id": "111111111111111111", "days": 7 });
    let (status, json) = delete_json(app, "/api/purge/audit-logs", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["deleted"], 7);
    assert_eq!(audit.purged.lock().unwrap()[0], ("111111111111111111".into(), 7));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_audit_logs_rejects_zero_days() {
    let (state, _, _, _) = build_state();
    let app = router::build_for_test(state);
    let body = serde_json::json!({ "guild_id": "111111111111111111", "days": 0 });
    let (status, json) = delete_json(app, "/api/purge/audit-logs", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains(">= 1"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_audit_logs_rejects_negative_days() {
    let (state, _, _, _) = build_state();
    let app = router::build_for_test(state);
    let body = serde_json::json!({ "guild_id": "111111111111111111", "days": -5 });
    let (status, _) = delete_json(app, "/api/purge/audit-logs", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// purge_logs (superadmin)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_logs_success_without_rbac() {
    // Sans rbac header -> pass-through (bot/internal)
    let (state, _, _, log) = build_state();
    let app = router::build_for_test(state);
    let body = serde_json::json!({ "days": 30 });
    let (status, json) = delete_json(app, "/api/purge/logs", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["deleted"], 99);
    assert_eq!(log.purged.lock().unwrap()[0], 30);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_logs_rejects_zero() {
    let (state, _, _, _) = build_state();
    let app = router::build_for_test(state);
    let body = serde_json::json!({ "days": 0 });
    let (status, json) = delete_json(app, "/api/purge/logs", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains(">= 1"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_logs_with_rbac_non_superadmin_forbidden() {
    use sentinel_api::adapters::inbound::http::middleware::rbac::Role;
    // User avec RBAC mais pas dans SUPERADMIN_USER_IDS -> forbidden
    let (state, _, _, _) = build_state();
    let app = router::build_for_test(state);
    let req = test_helpers::request_with_rbac(
        "DELETE", "/api/purge/logs",
        "444444444444444444", Some(Role::Owner), None,
        Some(serde_json::json!({"days": 30})),
    );
    let (status, json) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(json["error"].as_str().unwrap().contains("superadmin"));
}
