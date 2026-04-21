//! Tests d'integration HTTP pour les endpoints /api/bots/config et /api/bots/definitions.

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
use sentinel_api::domain::entities::{BotDefinition, BotGuildConfig};
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::outbound::BotConfigRepository;

use test_helpers::build_test_state_bot_config;

#[derive(Default)]
struct MockBotConfigRepo {
    definitions: Mutex<Vec<BotDefinition>>,
    configs: Mutex<Vec<BotGuildConfig>>,
    deleted: Mutex<Vec<(String, String, String)>>,
}

impl MockBotConfigRepo {
    fn new() -> Self { Self::default() }
    fn with_def(self, d: BotDefinition) -> Self { self.definitions.lock().unwrap().push(d); self }
    fn with_cfg(self, c: BotGuildConfig) -> Self { self.configs.lock().unwrap().push(c); self }
}

#[async_trait]
impl BotConfigRepository for MockBotConfigRepo {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
        Ok(self.definitions.lock().unwrap().clone())
    }
    async fn get_config(&self, guild_id: &str, bot_name: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(self.configs.lock().unwrap().iter()
            .filter(|c| c.guild_id == guild_id && c.bot_name == bot_name)
            .cloned().collect())
    }
    async fn get_all_config(&self, guild_id: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(self.configs.lock().unwrap().iter()
            .filter(|c| c.guild_id == guild_id)
            .cloned().collect())
    }
    async fn set_config(&self, guild_id: &str, bot_name: &str, config_key: &str, config_value: &str) -> Result<(), DomainError> {
        let mut cfg = self.configs.lock().unwrap();
        cfg.retain(|c| !(c.guild_id == guild_id && c.bot_name == bot_name && c.config_key == config_key));
        cfg.push(BotGuildConfig {
            id: Uuid::new_v4(),
            guild_id: guild_id.into(), bot_name: bot_name.into(),
            config_key: config_key.into(), config_value: config_value.into(),
            updated_at: Utc::now(),
        });
        Ok(())
    }
    async fn delete_config(&self, guild_id: &str, bot_name: &str, config_key: &str) -> Result<(), DomainError> {
        self.deleted.lock().unwrap().push((guild_id.into(), bot_name.into(), config_key.into()));
        let mut cfg = self.configs.lock().unwrap();
        cfg.retain(|c| !(c.guild_id == guild_id && c.bot_name == bot_name && c.config_key == config_key));
        Ok(())
    }
}

fn sample_def(name: &str) -> BotDefinition {
    BotDefinition {
        bot_name: name.into(),
        display_name: name.to_uppercase(),
        description: "desc".into(),
        config_schema: serde_json::json!({}),
    }
}

fn sample_cfg(guild_id: &str, bot: &str, k: &str, v: &str) -> BotGuildConfig {
    BotGuildConfig {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(), bot_name: bot.into(),
        config_key: k.into(), config_value: v.into(),
        updated_at: Utc::now(),
    }
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

async fn delete_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("DELETE").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

fn build_app(repo: Arc<MockBotConfigRepo>) -> axum::Router {
    router::build_for_test(build_test_state_bot_config(repo))
}

// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_definitions_returns_dtos() {
    let repo = MockBotConfigRepo::new().with_def(sample_def("moderation-bot"));
    let app = build_app(Arc::new(repo));
    let (status, json) = get(app, "/api/bots/definitions").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert!(!arr.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_guild_config_invalid_guild_422() {
    let app = build_app(Arc::new(MockBotConfigRepo::new()));
    let (status, _) = get(app, "/api/bots/config/not-a-snowflake").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_guild_config_returns_matching() {
    let repo = MockBotConfigRepo::new()
        .with_cfg(sample_cfg("111111111111111111", "moderation-bot", "threshold", "0.5"))
        .with_cfg(sample_cfg("222222222222222222", "other-bot", "x", "y"));
    let app = build_app(Arc::new(repo));
    let (status, json) = get(app, "/api/bots/config/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["bot_name"], "moderation-bot");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_bot_config_filters_by_bot_name() {
    let repo = MockBotConfigRepo::new()
        .with_cfg(sample_cfg("111111111111111111", "moderation-bot", "a", "1"))
        .with_cfg(sample_cfg("111111111111111111", "analytics-bot", "b", "2"));
    let app = build_app(Arc::new(repo));
    let (status, json) = get(app, "/api/bots/config/111111111111111111/moderation-bot").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["config_key"], "a");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_config_returns_204_and_persists() {
    let repo = Arc::new(MockBotConfigRepo::new());
    let app = build_app(repo.clone());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "bot_name": "moderation-bot",
        "config_key": "threshold",
        "config_value": "0.8"
    });
    let (status, _) = post_json(app, "/api/bots/config", body).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let cfgs = repo.configs.lock().unwrap();
    assert_eq!(cfgs.len(), 1);
    assert_eq!(cfgs[0].config_value, "0.8");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_config_upserts_on_same_key() {
    let repo = Arc::new(MockBotConfigRepo::new()
        .with_cfg(sample_cfg("111111111111111111", "b", "k", "old")));
    let app = build_app(repo.clone());
    let body = serde_json::json!({
        "guild_id": "111111111111111111", "bot_name": "b",
        "config_key": "k", "config_value": "new"
    });
    let (status, _) = post_json(app, "/api/bots/config", body).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let cfgs = repo.configs.lock().unwrap();
    assert_eq!(cfgs.len(), 1);
    assert_eq!(cfgs[0].config_value, "new");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_config_invalid_guild_422() {
    let app = build_app(Arc::new(MockBotConfigRepo::new()));
    let body = serde_json::json!({
        "guild_id": "abc", "bot_name": "b", "config_key": "k", "config_value": "v"
    });
    let (status, _) = post_json(app, "/api/bots/config", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_config_204_and_forwards_call() {
    let repo = Arc::new(MockBotConfigRepo::new()
        .with_cfg(sample_cfg("111111111111111111", "moderation-bot", "k", "v")));
    let app = build_app(repo.clone());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "bot_name": "moderation-bot",
        "config_key": "k"
    });
    let (status, _) = delete_json(app, "/api/bots/config", body).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let deleted = repo.deleted.lock().unwrap();
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].2, "k");
}
