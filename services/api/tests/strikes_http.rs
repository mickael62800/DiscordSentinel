//! Tests d'integration HTTP pour les endpoints strikes.

mod test_helpers;

use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::domain::entities::*;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::*;

use test_helpers::build_test_state_strikes;

// ══════════════════════════════════════════════════════════
// Mock Strikes Use Case
// ══════════════════════════════════════════════════════════

struct MockStrikesUC {
    strikes: Vec<UserStrike>,
    config: Option<StrikeConfig>,
    escalation_action: Option<String>,
    escalation_duration: Option<u64>,
}

impl MockStrikesUC {
    fn new() -> Self {
        Self {
            strikes: vec![],
            config: None,
            escalation_action: None,
            escalation_duration: None,
        }
    }

    fn with_strike(mut self, s: UserStrike) -> Self {
        self.strikes.push(s);
        self
    }

    #[allow(dead_code)]
    fn with_config(mut self, c: StrikeConfig) -> Self {
        self.config = Some(c);
        self
    }

    fn with_escalation(mut self, action: &str, duration: Option<u64>) -> Self {
        self.escalation_action = Some(action.into());
        self.escalation_duration = duration;
        self
    }
}

fn make_strike(guild_id: &str, user_id: &str, reason: &str, source: &str) -> UserStrike {
    UserStrike {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        user_id: user_id.into(),
        reason: reason.into(),
        source: source.into(),
        infraction_id: None,
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        created_at: Utc::now(),
    }
}

#[async_trait]
impl ManageStrikesUseCase for MockStrikesUC {
    async fn add_strike(&self, cmd: AddStrikeCommand) -> Result<StrikeResult, DomainError> {
        let strike = UserStrike {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            user_id: cmd.user_id,
            reason: cmd.reason,
            source: cmd.source,
            infraction_id: cmd.infraction_id,
            expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
            created_at: Utc::now(),
        };
        Ok(StrikeResult {
            strike,
            active_count: (self.strikes.len() + 1) as u32,
            escalation_action: self.escalation_action.clone(),
            escalation_duration: self.escalation_duration,
        })
    }

    async fn get_active_strikes(&self, guild_id: &str, user_id: &str) -> Result<Vec<UserStrike>, DomainError> {
        Ok(self.strikes
            .iter()
            .filter(|s| s.guild_id == guild_id && s.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn reset_strikes(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }

    async fn get_config(&self, guild_id: &str) -> Result<StrikeConfig, DomainError> {
        Ok(self.config.clone().unwrap_or_else(|| StrikeConfig::default_for_guild(guild_id)))
    }

    async fn save_config(&self, cmd: SaveStrikeConfigCommand) -> Result<StrikeConfig, DomainError> {
        Ok(StrikeConfig {
            guild_id: cmd.guild_id,
            window_secs: cmd.window_secs,
            thresholds: cmd.thresholds,
            enabled: cmd.enabled,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_app(uc: MockStrikesUC) -> axum::Router {
    let state = build_test_state_strikes(Arc::new(uc));
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

async fn put_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("PUT").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

async fn delete_req(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("DELETE").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null))
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — GET /api/strikes/config/{guild_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_config_returns_defaults() {
    let app = build_app(MockStrikesUC::new());
    let (status, json) = get(app, "/api/strikes/config/guild1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["guild_id"], "guild1");
    assert_eq!(json["window_secs"], 3600);
    assert!(json["thresholds"].as_array().unwrap().is_empty());
    assert_eq!(json["enabled"], true);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — PUT /api/strikes/config/{guild_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_config_success() {
    let app = build_app(MockStrikesUC::new());
    let body = serde_json::json!({
        "window_secs": 7200,
        "thresholds": [
            {"strikes": 3, "action": "mute", "duration": 600},
            {"strikes": 5, "action": "ban"}
        ],
        "enabled": true
    });
    let (status, json) = put_json(app, "/api/strikes/config/guild1", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["window_secs"], 7200);
    assert_eq!(json["thresholds"].as_array().unwrap().len(), 2);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — GET /api/strikes/{guild_id}/{user_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_strikes_empty() {
    let app = build_app(MockStrikesUC::new());
    let (status, json) = get(app, "/api/strikes/guild1/user1").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_strikes_with_data() {
    let uc = MockStrikesUC::new()
        .with_strike(make_strike("guild1", "user1", "Spam", "automod"))
        .with_strike(make_strike("guild1", "user1", "Insulte", "moderator"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/strikes/guild1/user1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 2);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — POST /api/strikes
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_strike_success() {
    let app = build_app(MockStrikesUC::new());
    let body = serde_json::json!({
        "guild_id": "guild1",
        "user_id": "user1",
        "reason": "Spam dans #general",
        "source": "automod"
    });
    let (status, json) = post_json(app, "/api/strikes", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["guild_id"], "guild1");
    assert_eq!(json["user_id"], "user1");
    assert_eq!(json["active_count"], 1);
    assert!(json["escalation_action"].is_null());
    assert!(json["id"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_strike_with_escalation() {
    let uc = MockStrikesUC::new()
        .with_strike(make_strike("guild1", "user1", "S1", "automod"))
        .with_strike(make_strike("guild1", "user1", "S2", "automod"))
        .with_escalation("mute", Some(600));
    let app = build_app(uc);
    let body = serde_json::json!({
        "guild_id": "guild1",
        "user_id": "user1",
        "reason": "3eme strike",
        "source": "automod"
    });
    let (status, json) = post_json(app, "/api/strikes", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["active_count"], 3);
    assert_eq!(json["escalation_action"], "mute");
    assert_eq!(json["escalation_duration"], 600);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_strike_missing_fields_returns_422() {
    let app = build_app(MockStrikesUC::new());
    let body = serde_json::json!({
        "guild_id": "guild1"
    });
    let (status, _) = post_json(app, "/api/strikes", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — DELETE /api/strikes/{guild_id}/{user_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_strikes_success() {
    let app = build_app(MockStrikesUC::new());
    let (status, json) = delete_req(app, "/api/strikes/guild1/user1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}
