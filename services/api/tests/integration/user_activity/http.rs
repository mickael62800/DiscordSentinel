//! Tests d'integration HTTP pour les endpoints user-activity.

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

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::domain::entities::UserActivity;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::outbound::UserActivityRepository;

use test_helpers::build_test_state_user_activity;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockUserActivityRepo {
    items: Mutex<Vec<UserActivity>>,
}

impl MockUserActivityRepo {
    fn new() -> Self { Self::default() }
    fn with(self, a: UserActivity) -> Self {
        self.items.lock().unwrap().push(a);
        self
    }
}

#[async_trait]
impl UserActivityRepository for MockUserActivityRepo {
    async fn create(&self, activity: &UserActivity) -> Result<(), DomainError> {
        self.items.lock().unwrap().push(activity.clone());
        Ok(())
    }
    async fn list(&self, guild_id: &str, user_id: &str, event_type: Option<&str>, limit: i64, offset: i64) -> Result<Vec<UserActivity>, DomainError> {
        let items = self.items.lock().unwrap();
        let matching: Vec<UserActivity> = items.iter()
            .filter(|a| a.guild_id == guild_id && a.user_id == user_id)
            .filter(|a| event_type.is_none_or(|e| a.event_type == e))
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(matching)
    }
}

fn build_app(repo: MockUserActivityRepo) -> axum::Router {
    router::build_for_test(build_test_state_user_activity(Arc::new(repo)))
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

fn sample_activity(guild_id: &str, user_id: &str, event_type: &str) -> UserActivity {
    UserActivity {
        id: uuid::Uuid::new_v4(),
        guild_id: guild_id.into(),
        user_id: user_id.into(),
        event_type: event_type.into(),
        channel_id: None,
        channel_name: None,
        content: None,
        metadata: serde_json::json!({}),
        created_at: Utc::now(),
    }
}

// ══════════════════════════════════════════════════════════
// POST /api/user-activity
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_activity_minimal() {
    let app = build_app(MockUserActivityRepo::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "u1",
        "event_type": "message_send"
    });
    let (status, json) = post_json(app, "/api/user-activity", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_activity_full_fields() {
    let app = build_app(MockUserActivityRepo::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "u1",
        "event_type": "voice_join",
        "channel_id": "c1",
        "channel_name": "general",
        "content": "hello",
        "metadata": {"source": "test"}
    });
    let (status, _) = post_json(app, "/api/user-activity", body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_activity_metadata_defaults_empty() {
    let app = build_app(MockUserActivityRepo::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "u1",
        "event_type": "message_send"
    });
    let (status, _) = post_json(app, "/api/user-activity", body).await;
    assert_eq!(status, StatusCode::OK);
}

// ══════════════════════════════════════════════════════════
// GET /api/user-activity/{guild_id}/{user_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_activity_empty() {
    let app = build_app(MockUserActivityRepo::new());
    let (status, json) = get(app, "/api/user-activity/111111111111111111/u1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_activity_scoped_to_user() {
    let repo = MockUserActivityRepo::new()
        .with(sample_activity("111111111111111111", "u1", "message_send"))
        .with(sample_activity("111111111111111111", "u2", "message_send"));
    let app = build_app(repo);
    let (status, json) = get(app, "/api/user-activity/111111111111111111/u1").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["user_id"], "u1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_activity_filter_by_event_type() {
    let repo = MockUserActivityRepo::new()
        .with(sample_activity("111111111111111111", "u1", "message_send"))
        .with(sample_activity("111111111111111111", "u1", "voice_join"));
    let app = build_app(repo);
    let (status, json) = get(app, "/api/user-activity/111111111111111111/u1?event_type=voice_join").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["event_type"], "voice_join");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_activity_limit_caps_at_200() {
    let mut repo = MockUserActivityRepo::new();
    for _ in 0..10 {
        repo = repo.with(sample_activity("111111111111111111", "u1", "message_send"));
    }
    let app = build_app(repo);
    let (status, json) = get(app, "/api/user-activity/111111111111111111/u1?limit=500").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 10);
}
