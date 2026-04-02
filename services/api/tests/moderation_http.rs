//! Tests d'integration HTTP pour les endpoints moderation.

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

// ══════════════════════════════════════════════════════════
// Mock Moderation Use Case
// ══════════════════════════════════════════════════════════

struct MockModerationUC {
    actions: Vec<ModerationAction>,
}

impl MockModerationUC {
    fn new() -> Self {
        Self { actions: vec![] }
    }

    fn with_action(mut self, a: ModerationAction) -> Self {
        self.actions.push(a);
        self
    }
}

fn make_action(
    id: Uuid,
    guild_id: &str,
    target_id: &str,
    target_name: &str,
    action_type: &str,
    reason: &str,
    gravity: Option<&str>,
    duration: Option<u64>,
) -> ModerationAction {
    ModerationAction {
        id,
        guild_id: guild_id.into(),
        channel_id: "chan1".into(),
        moderator_id: "mod1".into(),
        moderator_name: "ModeratorBob".into(),
        target_id: target_id.into(),
        target_name: target_name.into(),
        action_type: action_type.into(),
        reason: reason.into(),
        gravity: gravity.map(String::from),
        duration,
        created_at: Utc::now(),
    }
}

#[async_trait]
impl ManageModerationUseCase for MockModerationUC {
    async fn log_action(&self, cmd: LogModerationCommand) -> Result<ModerationAction, DomainError> {
        Ok(ModerationAction {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            channel_id: cmd.channel_id,
            moderator_id: cmd.moderator_id,
            moderator_name: cmd.moderator_name,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            action_type: cmd.action_type,
            reason: cmd.reason,
            gravity: cmd.gravity,
            duration: cmd.duration,
            created_at: Utc::now(),
        })
    }

    async fn get_history(&self, guild_id: &str, target_id: &str) -> Result<UserModerationHistory, DomainError> {
        let actions: Vec<ModerationAction> = self
            .actions
            .iter()
            .filter(|a| a.guild_id == guild_id && a.target_id == target_id)
            .cloned()
            .collect();

        if actions.is_empty() {
            return Ok(UserModerationHistory {
                target_id: target_id.into(),
                target_name: String::new(),
                total_warns: 0,
                total_mutes: 0,
                total_bans: 0,
                actions: vec![],
            });
        }

        let target_name = actions.first().map(|a| a.target_name.clone()).unwrap_or_default();
        let total_warns = actions.iter().filter(|a| a.action_type == "warn").count() as u32;
        let total_mutes = actions.iter().filter(|a| a.action_type.starts_with("mute")).count() as u32;
        let total_bans = actions.iter().filter(|a| a.action_type.starts_with("ban")).count() as u32;

        Ok(UserModerationHistory {
            target_id: target_id.into(),
            target_name,
            total_warns,
            total_mutes,
            total_bans,
            actions,
        })
    }

    async fn list_bans(&self, guild_id: Option<&str>, _limit: i64, _offset: i64) -> Result<Vec<ModerationAction>, DomainError> {
        let bans: Vec<ModerationAction> = self
            .actions
            .iter()
            .filter(|a| a.action_type.starts_with("ban"))
            .filter(|a| guild_id.map_or(true, |g| a.guild_id == g))
            .cloned()
            .collect();
        Ok(bans)
    }

    async fn delete_bans_for_user(&self, _guild_id: &str, _target_id: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_test_state_moderation(moderation_uc: Arc<dyn ManageModerationUseCase>) -> sentinel_api::adapters::inbound::http::state::AppState {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.moderation_uc = moderation_uc;
    state
}

fn build_app(uc: MockModerationUC) -> axum::Router {
    let state = build_test_state_moderation(Arc::new(uc));
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
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — POST /api/moderation/actions (log_action)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_action_warn_success() {
    let app = build_app(MockModerationUC::new());
    let body = serde_json::json!({
        "guild_id": "guild1",
        "channel_id": "chan1",
        "moderator_id": "mod1",
        "moderator_name": "Bob",
        "target_id": "user1",
        "target_name": "Alice",
        "action_type": "warn",
        "reason": "Spam dans #general",
        "gravity": "medium"
    });
    let (status, json) = post_json(app, "/api/moderation/actions", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["action_type"], "warn");
    assert_eq!(json["target_name"], "Alice");
    assert_eq!(json["reason"], "Spam dans #general");
    assert!(json["id"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_action_mute_temp_success() {
    let app = build_app(MockModerationUC::new());
    let body = serde_json::json!({
        "guild_id": "guild1",
        "channel_id": "chan1",
        "moderator_id": "mod1",
        "moderator_name": "Bob",
        "target_id": "user1",
        "target_name": "Alice",
        "action_type": "mute_temp",
        "reason": "Flood de mentions",
        "duration": 1800
    });
    let (status, json) = post_json(app, "/api/moderation/actions", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["action_type"], "mute_temp");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_action_ban_permanent_success() {
    let app = build_app(MockModerationUC::new());
    let body = serde_json::json!({
        "guild_id": "guild1",
        "channel_id": "chan1",
        "moderator_id": "mod1",
        "moderator_name": "Bob",
        "target_id": "user1",
        "target_name": "Alice",
        "action_type": "ban_permanent",
        "reason": "Harcelement repete"
    });
    let (status, json) = post_json(app, "/api/moderation/actions", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["action_type"], "ban_permanent");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_action_unmute_success() {
    let app = build_app(MockModerationUC::new());
    let body = serde_json::json!({
        "guild_id": "guild1",
        "channel_id": "chan1",
        "moderator_id": "mod1",
        "moderator_name": "Bob",
        "target_id": "user1",
        "target_name": "Alice",
        "action_type": "unmute",
        "reason": "Fin du mute"
    });
    let (status, json) = post_json(app, "/api/moderation/actions", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["action_type"], "unmute");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_action_unban_success() {
    let app = build_app(MockModerationUC::new());
    let body = serde_json::json!({
        "guild_id": "guild1",
        "channel_id": "chan1",
        "moderator_id": "mod1",
        "moderator_name": "Bob",
        "target_id": "user1",
        "target_name": "Alice",
        "action_type": "unban",
        "reason": "Pardon"
    });
    let (status, json) = post_json(app, "/api/moderation/actions", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["action_type"], "unban");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_action_missing_fields_returns_422() {
    let app = build_app(MockModerationUC::new());
    let body = serde_json::json!({
        "guild_id": "guild1",
        "action_type": "warn"
    });
    let (status, _) = post_json(app, "/api/moderation/actions", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — GET /api/moderation/history/{guild_id}/{user_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_history_empty() {
    let app = build_app(MockModerationUC::new());
    let (status, json) = get(app, "/api/moderation/history/guild1/user1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["target_id"], "user1");
    assert_eq!(json["total_warns"], 0);
    assert_eq!(json["total_mutes"], 0);
    assert_eq!(json["total_bans"], 0);
    assert!(json["actions"].as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_history_with_actions() {
    let uc = MockModerationUC::new()
        .with_action(make_action(Uuid::new_v4(), "guild1", "user1", "Alice", "warn", "Spam", Some("low"), None))
        .with_action(make_action(Uuid::new_v4(), "guild1", "user1", "Alice", "warn", "Insulte", Some("medium"), None))
        .with_action(make_action(Uuid::new_v4(), "guild1", "user1", "Alice", "mute_temp", "Flood", None, Some(600)));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/moderation/history/guild1/user1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["target_name"], "Alice");
    assert_eq!(json["total_warns"], 2);
    assert_eq!(json["total_mutes"], 1);
    assert_eq!(json["total_bans"], 0);
    assert_eq!(json["actions"].as_array().unwrap().len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_history_counts_ban_types() {
    let uc = MockModerationUC::new()
        .with_action(make_action(Uuid::new_v4(), "guild1", "user1", "Alice", "ban_temp", "Raid", None, Some(3600)))
        .with_action(make_action(Uuid::new_v4(), "guild1", "user1", "Alice", "ban_permanent", "Recidive", None, None));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/moderation/history/guild1/user1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["total_bans"], 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_history_different_guild_returns_empty() {
    let uc = MockModerationUC::new()
        .with_action(make_action(Uuid::new_v4(), "guild1", "user1", "Alice", "warn", "Spam", Some("low"), None));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/moderation/history/guild2/user1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["total_warns"], 0);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — GET /api/moderation/bans
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_bans_empty() {
    let app = build_app(MockModerationUC::new());
    let (status, json) = get(app, "/api/moderation/bans").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_bans_returns_only_bans() {
    let uc = MockModerationUC::new()
        .with_action(make_action(Uuid::new_v4(), "guild1", "user1", "Alice", "warn", "Spam", Some("low"), None))
        .with_action(make_action(Uuid::new_v4(), "guild1", "user2", "Bob", "ban_permanent", "Harcelement", None, None))
        .with_action(make_action(Uuid::new_v4(), "guild1", "user3", "Charlie", "ban_temp", "Raid", None, Some(7200)));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/moderation/bans").await;
    assert_eq!(status, StatusCode::OK);
    let bans = json.as_array().unwrap();
    assert_eq!(bans.len(), 2);
    assert!(bans.iter().all(|b| b["action_type"].as_str().unwrap().starts_with("ban")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_bans_filter_by_guild() {
    let uc = MockModerationUC::new()
        .with_action(make_action(Uuid::new_v4(), "guild1", "user1", "Alice", "ban_permanent", "Raid", None, None))
        .with_action(make_action(Uuid::new_v4(), "guild2", "user2", "Bob", "ban_permanent", "Spam", None, None));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/moderation/bans?guild_id=guild1").await;
    assert_eq!(status, StatusCode::OK);
    let bans = json.as_array().unwrap();
    assert_eq!(bans.len(), 1);
    assert_eq!(bans[0]["guild_id"], "guild1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_bans_no_filter_returns_all() {
    let uc = MockModerationUC::new()
        .with_action(make_action(Uuid::new_v4(), "guild1", "user1", "Alice", "ban_permanent", "Raid", None, None))
        .with_action(make_action(Uuid::new_v4(), "guild2", "user2", "Bob", "ban_temp", "Spam", None, Some(3600)));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/moderation/bans").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 2);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Ban DTO format
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ban_dto_has_all_fields() {
    let uc = MockModerationUC::new()
        .with_action(make_action(Uuid::new_v4(), "guild1", "user1", "Alice", "ban_permanent", "Harcelement", None, None));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/moderation/bans").await;
    assert_eq!(status, StatusCode::OK);
    let ban = &json[0];
    assert!(ban["id"].as_str().is_some());
    assert_eq!(ban["guild_id"], "guild1");
    assert_eq!(ban["target_id"], "user1");
    assert_eq!(ban["target_name"], "Alice");
    assert_eq!(ban["moderator_name"], "ModeratorBob");
    assert_eq!(ban["action_type"], "ban_permanent");
    assert_eq!(ban["reason"], "Harcelement");
    assert!(ban["created_at"].as_str().is_some());
}
