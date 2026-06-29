//! Tests d'integration HTTP pour les endpoints watched_users.

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

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::ports::inbound::audit::manage_watched_users::ManageWatchedUsersUseCase;
use sentinel_api::ports::inbound::audit::manage_watched_users::UserDossier;
use sentinel_core::domain::entities::audit::watched_user::WatchedUser;
use sentinel_core::domain::errors::DomainError;
use test_helpers::build_test_state_watched_users;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockWatchedUsersUC {
    users: Mutex<Vec<WatchedUser>>,
    added: Mutex<Vec<(String, String, String, String)>>,
    removed: Mutex<Vec<(String, String)>>,
    dossier_not_found: bool,
}

impl MockWatchedUsersUC {
    fn new() -> Self {
        Self::default()
    }
    fn with_user(self, u: WatchedUser) -> Self {
        self.users.lock().unwrap().push(u);
        self
    }
    fn dossier_fails(mut self) -> Self {
        self.dossier_not_found = true;
        self
    }
}

#[async_trait]
impl ManageWatchedUsersUseCase for MockWatchedUsersUC {
    async fn list_watched_users(
        &self,
        guild_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WatchedUser>, DomainError> {
        let users = self.users.lock().unwrap();
        let matching: Vec<WatchedUser> = users
            .iter()
            .filter(|u| guild_id.is_none_or(|g| u.guild_id == g))
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(matching)
    }
    async fn get_user_dossier(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UserDossier, DomainError> {
        if self.dossier_not_found {
            return Err(DomainError::NotFound("user".into()));
        }
        let users = self.users.lock().unwrap();
        let user = users
            .iter()
            .find(|u| u.guild_id == guild_id && u.user_id == user_id)
            .cloned()
            .ok_or_else(|| DomainError::NotFound("user".into()))?;
        Ok(UserDossier {
            user,
            infractions: vec![],
            moderation_actions: vec![],
            security_events: vec![],
            notes: vec![],
        })
    }
    async fn add_manual_watch(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        reason: &str,
    ) -> Result<(), DomainError> {
        self.added.lock().unwrap().push((
            guild_id.into(),
            user_id.into(),
            username.into(),
            reason.into(),
        ));
        Ok(())
    }
    async fn remove_manual_watch(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.removed
            .lock()
            .unwrap()
            .push((guild_id.into(), user_id.into()));
        Ok(())
    }
}

fn build_app(uc: MockWatchedUsersUC) -> axum::Router {
    router::build_for_test(build_test_state_watched_users(Arc::new(uc)))
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

fn sample_user(guild_id: &str, user_id: &str, risk: &str) -> WatchedUser {
    WatchedUser {
        user_id: user_id.into(),
        username: "alice".into(),
        guild_id: guild_id.into(),
        guild_name: "Guild".into(),
        risk_level: risk.into(),
        total_warns: 1,
        total_mutes: 1,
        total_bans: 0,
        last_incident_at: None,
        security_events_count: 2,
        first_seen_at: Utc::now(),
    }
}

// ══════════════════════════════════════════════════════════
// GET /api/watched-users
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_watched_users_empty() {
    let app = build_app(MockWatchedUsersUC::new());
    let (status, json) = get(app, "/api/watched-users").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_watched_users_filter_by_guild() {
    let uc = MockWatchedUsersUC::new()
        .with_user(sample_user("111111111111111111", "u1", "high"))
        .with_user(sample_user("222222222222222222", "u2", "low"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/watched-users?guild_id=111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["risk_level"], "high");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_watched_users_respects_limit() {
    let mut uc = MockWatchedUsersUC::new();
    for i in 0..5 {
        uc = uc.with_user(sample_user("111111111111111111", &format!("u{i}"), "low"));
    }
    let app = build_app(uc);
    let (status, json) = get(app, "/api/watched-users?limit=3").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 3);
}

// ══════════════════════════════════════════════════════════
// GET /api/watched-users/{guild_id}/{user_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_dossier_returns_user_with_empty_relations() {
    let uc =
        MockWatchedUsersUC::new().with_user(sample_user("111111111111111111", "u1", "critical"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/watched-users/111111111111111111/u1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["user"]["user_id"], "u1");
    assert_eq!(json["user"]["risk_level"], "critical");
    assert_eq!(json["infractions"], serde_json::json!([]));
    assert_eq!(json["notes"], serde_json::json!([]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_dossier_not_found_returns_404() {
    let app = build_app(MockWatchedUsersUC::new().dossier_fails());
    let (status, _) = get(app, "/api/watched-users/111111111111111111/u1").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ══════════════════════════════════════════════════════════
// POST /api/watched-users
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_watched_user_success() {
    let app = build_app(MockWatchedUsersUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "u1",
        "username": "alice",
        "reason": "Comportement suspect"
    });
    let (status, json) = post_json(app, "/api/watched-users", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_watched_user_reason_defaults_empty() {
    let app = build_app(MockWatchedUsersUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "u1",
        "username": "alice"
    });
    let (status, _) = post_json(app, "/api/watched-users", body).await;
    assert_eq!(status, StatusCode::OK);
}

// ══════════════════════════════════════════════════════════
// DELETE /api/watched-users/{guild_id}/{user_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remove_watched_user_success_no_rbac_header() {
    // Sans X-Discord-Token → check_role_for_guild pass-through.
    let app = build_app(MockWatchedUsersUC::new());
    let (status, json) = delete(app, "/api/watched-users/111111111111111111/u1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}
