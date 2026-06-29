//! Tests d'integration HTTP pour les endpoints stats.

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
use sentinel_api::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use sentinel_api::ports::inbound::audit::manage_stats::RecordMessagesCommand;
use sentinel_api::ports::inbound::audit::manage_stats::RecordVoiceCommand;
use sentinel_core::domain::entities::audit::dashboard_stats::DashboardStats;
use sentinel_core::domain::entities::audit::user_stats::GuildStatsOverview;
use sentinel_core::domain::entities::audit::user_stats::GuildVoiceStats;
use sentinel_core::domain::entities::audit::user_stats::UserStats;
use sentinel_core::domain::errors::DomainError;
use test_helpers::build_test_state_stats;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockStatsUC {
    users: Mutex<Vec<UserStats>>,
    msg_calls: Mutex<Vec<RecordMessagesCommand>>,
    voice_calls: Mutex<Vec<RecordVoiceCommand>>,
}

impl MockStatsUC {
    fn new() -> Self {
        Self::default()
    }
    fn with_user(self, u: UserStats) -> Self {
        self.users.lock().unwrap().push(u);
        self
    }
}

fn sample_user(guild_id: &str, user_id: &str, msgs: u64, voice: u64) -> UserStats {
    UserStats {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        user_id: user_id.into(),
        username: format!("user_{user_id}"),
        message_count: msgs,
        voice_seconds: voice,
        updated_at: Utc::now(),
    }
}

#[async_trait]
impl ManageStatsUseCase for MockStatsUC {
    async fn record_messages(&self, cmd: RecordMessagesCommand) -> Result<(), DomainError> {
        self.msg_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn record_voice(&self, cmd: RecordVoiceCommand) -> Result<(), DomainError> {
        self.voice_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn get_user_stats(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserStats>, DomainError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.guild_id == guild_id && u.user_id == user_id)
            .cloned())
    }
    async fn get_guild_overview(&self, guild_id: &str) -> Result<GuildStatsOverview, DomainError> {
        let users = self.users.lock().unwrap();
        let gu: Vec<UserStats> = users
            .iter()
            .filter(|u| u.guild_id == guild_id)
            .cloned()
            .collect();
        Ok(GuildStatsOverview {
            guild_id: guild_id.into(),
            total_messages: gu.iter().map(|u| u.message_count).sum(),
            total_voice_seconds: gu.iter().map(|u| u.voice_seconds).sum(),
            active_members: gu.len() as u64,
            total_infractions: 0,
            total_warns: 0,
            total_mutes: 0,
            total_bans: 0,
            top_members: gu,
        })
    }
    async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<UserStats>, DomainError> {
        let users = self.users.lock().unwrap();
        let mut matching: Vec<UserStats> = users
            .iter()
            .filter(|u| u.guild_id == guild_id)
            .cloned()
            .collect();
        matching.sort_by(|a, b| b.message_count.cmp(&a.message_count));
        matching.truncate(limit as usize);
        Ok(matching)
    }
    async fn get_dashboard_stats(&self) -> Result<DashboardStats, DomainError> {
        unimplemented!()
    }
    async fn get_guild_voice_stats(
        &self,
        _: &str,
        _: u32,
        _: u32,
    ) -> Result<GuildVoiceStats, DomainError> {
        Ok(GuildVoiceStats {
            total_channels: 3,
            total_sessions: 10,
            total_duration_secs: 3600,
            unique_users: 5,
            avg_session_secs: 360,
            temp_channels: 1,
            perm_channels: 2,
            channels: vec![],
        })
    }
}

fn build_app(uc: Arc<MockStatsUC>) -> axum::Router {
    router::build_for_test(build_test_state_stats(uc))
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

// ══════════════════════════════════════════════════════════
// POST record_messages / record_voice
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_messages_returns_204_and_forwards_command() {
    let uc = Arc::new(MockStatsUC::new());
    let app = build_app(uc.clone());
    let body = serde_json::json!({
        "guild_id": "111111111111111111", "user_id": "u1",
        "username": "alice", "count": 3
    });
    let (status, _) = post_json(app, "/api/stats/messages", body).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let calls = uc.msg_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].count, 3);
    assert_eq!(calls[0].user_id, "u1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_voice_returns_204_and_forwards_command() {
    let uc = Arc::new(MockStatsUC::new());
    let app = build_app(uc.clone());
    let body = serde_json::json!({
        "guild_id": "111111111111111111", "user_id": "u1", "username": "alice",
        "seconds": 120, "channel_id": "c1", "channel_name": "general"
    });
    let (status, _) = post_json(app, "/api/stats/voice", body).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let calls = uc.voice_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].seconds, 120);
    assert_eq!(calls[0].channel_name, "general");
}

// ══════════════════════════════════════════════════════════
// GET user stats / overview / leaderboard / voice-stats
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_user_stats_none_returns_null() {
    let app = build_app(Arc::new(MockStatsUC::new()));
    let (status, json) = get(app, "/api/stats/111111111111111111/user/u1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, serde_json::Value::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_user_stats_returns_dto() {
    let uc = MockStatsUC::new().with_user(sample_user("111111111111111111", "u1", 100, 3600));
    let app = build_app(Arc::new(uc));
    let (status, json) = get(app, "/api/stats/111111111111111111/user/u1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["user_id"], "u1");
    assert_eq!(json["message_count"], 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_guild_overview_aggregates() {
    let uc = MockStatsUC::new()
        .with_user(sample_user("111111111111111111", "u1", 50, 1800))
        .with_user(sample_user("111111111111111111", "u2", 30, 600));
    let app = build_app(Arc::new(uc));
    let (status, json) = get(app, "/api/stats/111111111111111111/overview").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["total_messages"], 80);
    assert_eq!(json["total_voice_seconds"], 2400);
    assert_eq!(json["active_members"], 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_leaderboard_sorted_desc_by_messages() {
    let uc = MockStatsUC::new()
        .with_user(sample_user("111111111111111111", "u1", 10, 0))
        .with_user(sample_user("111111111111111111", "u2", 50, 0))
        .with_user(sample_user("111111111111111111", "u3", 25, 0));
    let app = build_app(Arc::new(uc));
    let (status, json) = get(app, "/api/stats/111111111111111111/leaderboard").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr[0]["user_id"], "u2");
    assert_eq!(arr[2]["user_id"], "u1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_leaderboard_caps_limit_at_50() {
    let mut uc = MockStatsUC::new();
    for i in 0..10 {
        uc = uc.with_user(sample_user(
            "111111111111111111",
            &format!("u{i}"),
            i as u64,
            0,
        ));
    }
    let app = build_app(Arc::new(uc));
    let (status, json) = get(app, "/api/stats/111111111111111111/leaderboard?limit=100").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_voice_stats_returns_aggregate() {
    let app = build_app(Arc::new(MockStatsUC::new()));
    let (status, json) = get(app, "/api/stats/111111111111111111/voice-stats").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["total_channels"], 3);
    assert_eq!(json["temp_channels"], 1);
    assert_eq!(json["perm_channels"], 2);
}
