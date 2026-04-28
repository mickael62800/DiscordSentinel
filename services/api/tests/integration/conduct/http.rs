//! Tests d'integration HTTP pour les endpoints conduct.

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
use sentinel_api::domain::entities::community::conduct::ConductConfig;
use sentinel_api::domain::entities::community::conduct::ConductPointsLog;
use sentinel_api::domain::entities::community::conduct::UserConductPoints;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::community::manage_conduct::AddPointsCommand;
use sentinel_api::ports::inbound::community::manage_conduct::DeductPointsCommand;
use sentinel_api::ports::inbound::community::manage_conduct::ManageConductUseCase;
use sentinel_api::ports::inbound::community::manage_conduct::SaveConductConfigCommand;
use test_helpers::build_test_state_conduct;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockConductUC {
    points: Mutex<Vec<UserConductPoints>>,
    config: Mutex<Option<ConductConfig>>,
}

impl MockConductUC {
    fn new() -> Self { Self::default() }
    fn with_points(self, p: UserConductPoints) -> Self {
        self.points.lock().unwrap().push(p);
        self
    }
    fn with_config(self, c: ConductConfig) -> Self {
        *self.config.lock().unwrap() = Some(c);
        self
    }
}

fn default_config(guild_id: &str) -> ConductConfig {
    let now = Utc::now();
    ConductConfig {
        guild_id: guild_id.into(),
        max_points: 100,
        regen_amount: 1,
        regen_interval: "hour".into(),
        penalty_warn: 5,
        penalty_delete: 10,
        penalty_mute: 25,
        penalty_ban: 100,
        created_at: now,
        updated_at: now,
    }
}

#[async_trait]
impl ManageConductUseCase for MockConductUC {
    async fn get_config(&self, guild_id: &str) -> Result<ConductConfig, DomainError> {
        Ok(self.config.lock().unwrap().clone().unwrap_or_else(|| default_config(guild_id)))
    }
    async fn save_config(&self, cmd: SaveConductConfigCommand) -> Result<ConductConfig, DomainError> {
        let now = Utc::now();
        let cfg = ConductConfig {
            guild_id: cmd.guild_id,
            max_points: cmd.max_points,
            regen_amount: cmd.regen_amount,
            regen_interval: cmd.regen_interval,
            penalty_warn: cmd.penalty_warn,
            penalty_delete: cmd.penalty_delete,
            penalty_mute: cmd.penalty_mute,
            penalty_ban: cmd.penalty_ban,
            created_at: now,
            updated_at: now,
        };
        *self.config.lock().unwrap() = Some(cfg.clone());
        Ok(cfg)
    }
    async fn get_points(&self, guild_id: &str, user_id: &str) -> Result<UserConductPoints, DomainError> {
        let pts = self.points.lock().unwrap();
        Ok(pts.iter().find(|p| p.guild_id == guild_id && p.user_id == user_id).cloned()
            .unwrap_or_else(|| {
                let now = Utc::now();
                UserConductPoints {
                    id: Uuid::new_v4(),
                    guild_id: guild_id.into(),
                    user_id: user_id.into(),
                    username: String::new(),
                    points: 100,
                    last_regen_at: now,
                    created_at: now,
                    updated_at: now,
                }
            }))
    }
    async fn deduct_points(&self, _: DeductPointsCommand) -> Result<UserConductPoints, DomainError> {
        unimplemented!()
    }
    async fn add_points(&self, cmd: AddPointsCommand) -> Result<UserConductPoints, DomainError> {
        let mut pts = self.points.lock().unwrap();
        let entry = pts.iter_mut()
            .find(|p| p.guild_id == cmd.guild_id && p.user_id == cmd.user_id);
        match entry {
            Some(p) => { p.points += cmd.amount; Ok(p.clone()) }
            None => {
                let now = Utc::now();
                let new = UserConductPoints {
                    id: Uuid::new_v4(),
                    guild_id: cmd.guild_id,
                    user_id: cmd.user_id,
                    username: String::new(),
                    points: 100 + cmd.amount,
                    last_regen_at: now,
                    created_at: now,
                    updated_at: now,
                };
                pts.push(new.clone());
                Ok(new)
            }
        }
    }
    async fn get_leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<UserConductPoints>, DomainError> {
        let pts = self.points.lock().unwrap();
        let mut matching: Vec<UserConductPoints> = pts.iter()
            .filter(|p| p.guild_id == guild_id).cloned().collect();
        matching.sort_by(|a, b| b.points.cmp(&a.points));
        matching.truncate(limit as usize);
        Ok(matching)
    }
    async fn get_points_log(&self, _guild_id: &str, _user_id: &str, _limit: i64) -> Result<Vec<ConductPointsLog>, DomainError> {
        Ok(vec![ConductPointsLog {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            user_id: "u".into(),
            delta: -5,
            reason: "warn".into(),
            points_before: 100,
            points_after: 95,
            created_at: Utc::now(),
        }])
    }
    async fn run_regen(&self) -> Result<u64, DomainError> { Ok(0) }
}

fn build_app(uc: MockConductUC) -> axum::Router {
    router::build_for_test(build_test_state_conduct(Arc::new(uc)))
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

fn sample_points(guild_id: &str, user_id: &str, points: i32) -> UserConductPoints {
    let now = Utc::now();
    UserConductPoints {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        user_id: user_id.into(),
        username: "alice".into(),
        points,
        last_regen_at: now,
        created_at: now,
        updated_at: now,
    }
}

// ══════════════════════════════════════════════════════════
// GET /api/conduct/config/{guild_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_config_returns_defaults() {
    let app = build_app(MockConductUC::new());
    let (status, json) = get(app, "/api/conduct/config/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["guild_id"], "111111111111111111");
    assert_eq!(json["max_points"], 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_config_returns_stored() {
    let mut cfg = default_config("111111111111111111");
    cfg.max_points = 500;
    let app = build_app(MockConductUC::new().with_config(cfg));
    let (status, json) = get(app, "/api/conduct/config/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["max_points"], 500);
}

// ══════════════════════════════════════════════════════════
// POST /api/conduct/config
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_config_persists_and_returns() {
    let app = build_app(MockConductUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "max_points": 200,
        "regen_amount": 2,
        "regen_interval": "hour",
        "penalty_warn": 5,
        "penalty_delete": 10,
        "penalty_mute": 25,
        "penalty_ban": 100
    });
    let (status, json) = post_json(app, "/api/conduct/config", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["max_points"], 200);
    assert_eq!(json["regen_amount"], 2);
}

// ══════════════════════════════════════════════════════════
// GET /api/conduct/{guild_id}/{user_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_points_returns_stored() {
    let uc = MockConductUC::new().with_points(sample_points("111111111111111111", "u1", 75));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/conduct/111111111111111111/u1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["user_id"], "u1");
    assert_eq!(json["points"], 75);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_points_returns_full_when_not_stored() {
    let app = build_app(MockConductUC::new());
    let (status, json) = get(app, "/api/conduct/111111111111111111/newuser").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["points"], 100);
}

// ══════════════════════════════════════════════════════════
// GET /api/conduct/{guild_id}/leaderboard
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_sorted_desc_by_points() {
    let uc = MockConductUC::new()
        .with_points(sample_points("111111111111111111", "u1", 30))
        .with_points(sample_points("111111111111111111", "u2", 90))
        .with_points(sample_points("111111111111111111", "u3", 60));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/conduct/111111111111111111/leaderboard").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["user_id"], "u2");
    assert_eq!(arr[2]["user_id"], "u1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_respects_limit_cap() {
    let mut uc = MockConductUC::new();
    for i in 0..10 {
        uc = uc.with_points(sample_points("111111111111111111", &format!("u{i}"), i));
    }
    let app = build_app(uc);
    let (status, json) = get(app, "/api/conduct/111111111111111111/leaderboard?limit=3").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 3);
}

// ══════════════════════════════════════════════════════════
// GET /api/conduct/{guild_id}/{user_id}/log
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_points_log_returns_entries() {
    let app = build_app(MockConductUC::new());
    let (status, json) = get(app, "/api/conduct/111111111111111111/u1/log").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["delta"], -5);
}

// ══════════════════════════════════════════════════════════
// POST /api/conduct/{guild_id}/{user_id}/add
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_points_increments() {
    let uc = MockConductUC::new().with_points(sample_points("111111111111111111", "u1", 50));
    let app = build_app(uc);
    let body = serde_json::json!({"amount": 20, "reason": "bon comportement"});
    let (status, json) = post_json(app, "/api/conduct/111111111111111111/u1/add", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["points"], 70);
}
