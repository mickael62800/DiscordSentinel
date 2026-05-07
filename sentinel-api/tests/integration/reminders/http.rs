//! Tests d'integration HTTP pour les endpoints reminders.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use chrono::Duration;
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_core::domain::entities::moderation::action::sanction_reminder::*;
use sentinel_core::domain::errors::DomainError;
use sentinel_api::ports::inbound::moderation::manage_reminders::*;

// ══════════════════════════════════════════════════════════
// Mock Reminders Use Case
// ══════════════════════════════════════════════════════════

struct MockRemindersUC {
    reminders: Vec<SanctionReminder>,
}

impl MockRemindersUC {
    fn new() -> Self {
        Self { reminders: vec![] }
    }

    fn with_reminder(mut self, r: SanctionReminder) -> Self {
        self.reminders.push(r);
        self
    }
}

fn make_reminder(guild_id: &str, status: &str) -> SanctionReminder {
    SanctionReminder {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        moderator_id: "mod1".into(),
        moderator_name: "Bob".into(),
        target_id: "u1".into(),
        target_name: "Alice".into(),
        action_type: "mute_temp".into(),
        reason: "Spam".into(),
        action_id: Uuid::new_v4(),
        remind_at: Utc::now() - Duration::minutes(5),
        expires_at: Utc::now() + Duration::hours(1),
        status: status.into(),
        created_at: Utc::now(),
    }
}

#[async_trait]
impl ManageRemindersUseCase for MockRemindersUC {
    async fn create_reminder(&self, cmd: CreateReminderCommand) -> Result<SanctionReminder, DomainError> {
        if cmd.duration_secs <= cmd.remind_before_secs {
            return Err(DomainError::ValidationError("Duree trop courte".into()));
        }
        Ok(SanctionReminder {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            moderator_id: cmd.moderator_id,
            moderator_name: cmd.moderator_name,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            action_type: cmd.action_type,
            reason: cmd.reason,
            action_id: cmd.action_id,
            remind_at: Utc::now() + Duration::seconds(cmd.duration_secs as i64 - cmd.remind_before_secs as i64),
            expires_at: Utc::now() + Duration::seconds(cmd.duration_secs as i64),
            status: "pending".into(),
            created_at: Utc::now(),
        })
    }

    async fn get_pending_reminders(&self) -> Result<Vec<SanctionReminder>, DomainError> {
        Ok(self.reminders.iter().filter(|r| r.status == "pending").cloned().collect())
    }

    async fn mark_sent(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
    async fn cancel_for_action(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }

    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<SanctionReminder>, DomainError> {
        Ok(self.reminders.iter().filter(|r| r.guild_id == guild_id).cloned().collect())
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_app(uc: MockRemindersUC) -> axum::Router {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.reminders_uc = Arc::new(uc);
    router::build_for_test(state)
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null))
}

async fn post_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("POST").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — POST /api/reminders
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_reminder_success() {
    let app = build_app(MockRemindersUC::new());
    let body = serde_json::json!({
        "guild_id": "guild1",
        "moderator_id": "mod1",
        "moderator_name": "Bob",
        "target_id": "user1",
        "target_name": "Alice",
        "action_type": "mute_temp",
        "reason": "Spam",
        "action_id": Uuid::new_v4().to_string(),
        "duration_secs": 7200,
        "remind_before_secs": 3600
    });
    let (status, json) = post_json(app, "/api/reminders", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["guild_id"], "guild1");
    assert_eq!(json["status"], "pending");
    assert!(json["id"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_reminder_too_short_returns_422() {
    let app = build_app(MockRemindersUC::new());
    let body = serde_json::json!({
        "guild_id": "guild1",
        "moderator_id": "mod1",
        "moderator_name": "Bob",
        "target_id": "user1",
        "target_name": "Alice",
        "action_type": "mute_temp",
        "reason": "Spam",
        "action_id": Uuid::new_v4().to_string(),
        "duration_secs": 1800,
        "remind_before_secs": 3600
    });
    let (status, _) = post_json(app, "/api/reminders", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — GET /api/reminders/pending
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_pending_empty() {
    let app = build_app(MockRemindersUC::new());
    let (status, json) = get(app, "/api/reminders/pending").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_pending_with_data() {
    let uc = MockRemindersUC::new()
        .with_reminder(make_reminder("guild1", "pending"))
        .with_reminder(make_reminder("guild1", "sent"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/reminders/pending").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — GET /api/reminders/{guild_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_by_guild_empty() {
    let app = build_app(MockRemindersUC::new());
    let (status, json) = get(app, "/api/reminders/guild1").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_by_guild_with_data() {
    let uc = MockRemindersUC::new()
        .with_reminder(make_reminder("guild1", "pending"))
        .with_reminder(make_reminder("guild2", "pending"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/reminders/guild1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
}
