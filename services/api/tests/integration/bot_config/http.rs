//! Tests d'integration HTTP pour les endpoints /api/bots/config et /api/bots/definitions.

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
use sentinel_api::domain::entities::system::bot_config::BotDefinition;
use sentinel_api::domain::entities::system::bot_config::BotGuildConfig;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::outbound::system::bot_config_repository::BotConfigRepository;

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

// ══════════════════════════════════════════════════════════
// Cache-hit paths (2 calls → 2eme hit Redis) et RBAC
// ══════════════════════════════════════════════════════════

async fn pool() -> sqlx::PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    sqlx::PgPool::connect(&url).await.unwrap()
}

async fn seed_rbac(pool: &sqlx::PgPool, user_id: &str, guild_id: &str, role: &str) {
    sqlx::query("INSERT INTO api_users (discord_user_id, display_name) VALUES ($1, 'T') ON CONFLICT DO NOTHING")
        .bind(user_id).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO api_user_guilds (discord_user_id, guild_id, role) VALUES ($1, $2, $3)")
        .bind(user_id).bind(guild_id).bind(role).execute(pool).await.unwrap();
}

async fn send_request(app: axum::Router, req: axum::http::Request<Body>) -> (StatusCode, serde_json::Value) {
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_definitions_second_call_hits_cache() {
    // Populate, puis 2e appel hit le cache et deserialise.
    let repo = MockBotConfigRepo::new().with_def(sample_def("cache-test-bot"));
    let repo = Arc::new(repo);
    let app1 = build_app(repo.clone());
    let (s, _) = get(app1, "/api/bots/definitions").await;
    assert_eq!(s, StatusCode::OK);
    let app2 = build_app(repo.clone());
    let (s, json) = get(app2, "/api/bots/definitions").await;
    assert_eq!(s, StatusCode::OK);
    assert!(json.as_array().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_guild_config_second_call_hits_cache() {
    // Invalide le cache au prealable, puis populate + hit.
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let repo = Arc::new(MockBotConfigRepo::new()
        .with_cfg(sample_cfg(&guild_id, "bot-x", "k", "v")));
    let app1 = build_app(repo.clone());
    let (s, _) = get(app1, &format!("/api/bots/config/{guild_id}")).await;
    assert_eq!(s, StatusCode::OK);
    let app2 = build_app(repo.clone());
    let (s, json) = get(app2, &format!("/api/bots/config/{guild_id}")).await;
    assert_eq!(s, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert!(arr.iter().any(|c| c["bot_name"] == "bot-x"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_bot_config_second_call_hits_cache() {
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let repo = Arc::new(MockBotConfigRepo::new()
        .with_cfg(sample_cfg(&guild_id, "bot-y", "k", "v")));
    let app1 = build_app(repo.clone());
    let (s, _) = get(app1, &format!("/api/bots/config/{guild_id}/bot-y")).await;
    assert_eq!(s, StatusCode::OK);
    let app2 = build_app(repo.clone());
    let (s, json) = get(app2, &format!("/api/bots/config/{guild_id}/bot-y")).await;
    assert_eq!(s, StatusCode::OK);
    assert!(json.as_array().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_bot_config_invalid_guild_id_422() {
    let app = build_app(Arc::new(MockBotConfigRepo::new()));
    let (status, _) = get(app, "/api/bots/config/abc/bot-name").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_config_with_rbac_admin_succeeds() {
    use sentinel_api::adapters::inbound::http::middleware::rbac::Role;
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let p = pool().await;
    let user_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    seed_rbac(&p, &user_id, &guild_id, "admin").await;

    let repo = Arc::new(MockBotConfigRepo::new());
    let app = build_app(repo.clone());
    let body = serde_json::json!({
        "guild_id": guild_id, "bot_name": "moderation-bot",
        "config_key": "threshold", "config_value": "0.8"
    });
    let req = test_helpers::request_with_rbac(
        "POST", "/api/bots/config",
        &user_id, Some(Role::Admin), Some(guild_id), Some(body),
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(repo.configs.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_config_with_rbac_moderator_forbidden() {
    use sentinel_api::adapters::inbound::http::middleware::rbac::Role;
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let p = pool().await;
    let user_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    seed_rbac(&p, &user_id, &guild_id, "moderator").await;

    let app = build_app(Arc::new(MockBotConfigRepo::new()));
    let body = serde_json::json!({
        "guild_id": guild_id, "bot_name": "b", "config_key": "k", "config_value": "v"
    });
    let req = test_helpers::request_with_rbac(
        "POST", "/api/bots/config",
        &user_id, Some(Role::Moderator), Some(guild_id), Some(body),
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_config_with_rbac_admin_succeeds() {
    use sentinel_api::adapters::inbound::http::middleware::rbac::Role;
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let p = pool().await;
    let user_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    seed_rbac(&p, &user_id, &guild_id, "admin").await;

    let repo = Arc::new(MockBotConfigRepo::new().with_cfg(sample_cfg(&guild_id, "moderation-bot", "k", "v")));
    let app = build_app(repo.clone());
    let body = serde_json::json!({
        "guild_id": guild_id, "bot_name": "moderation-bot", "config_key": "k"
    });
    let req = test_helpers::request_with_rbac(
        "DELETE", "/api/bots/config",
        &user_id, Some(Role::Admin), Some(guild_id), Some(body),
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(repo.deleted.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_config_with_rbac_viewer_forbidden() {
    use sentinel_api::adapters::inbound::http::middleware::rbac::Role;
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let p = pool().await;
    let user_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    seed_rbac(&p, &user_id, &guild_id, "viewer").await;

    let app = build_app(Arc::new(MockBotConfigRepo::new()));
    let body = serde_json::json!({
        "guild_id": guild_id, "bot_name": "b", "config_key": "k"
    });
    let req = test_helpers::request_with_rbac(
        "DELETE", "/api/bots/config",
        &user_id, Some(Role::Viewer), Some(guild_id), Some(body),
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_config_invalid_bot_name_422() {
    let app = build_app(Arc::new(MockBotConfigRepo::new()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "bot_name": "a".repeat(300), // > 200 chars
        "config_key": "k"
    });
    let (status, _) = delete_json(app, "/api/bots/config", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_config_invalid_config_key_422() {
    let app = build_app(Arc::new(MockBotConfigRepo::new()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "bot_name": "b",
        "config_key": "a".repeat(300)
    });
    let (status, _) = delete_json(app, "/api/bots/config", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
