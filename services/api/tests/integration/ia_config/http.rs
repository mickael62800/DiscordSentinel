//! Tests d'integration HTTP pour les endpoints ia-config.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::domain::entities::IaConfig;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::outbound::IaConfigRepository;

use test_helpers::build_test_state_ia_config;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockIaConfigRepo {
    stored: Mutex<Option<IaConfig>>,
}

impl MockIaConfigRepo {
    fn new() -> Self { Self::default() }
    fn with(self, c: IaConfig) -> Self {
        *self.stored.lock().unwrap() = Some(c);
        self
    }
}

#[async_trait]
impl IaConfigRepository for MockIaConfigRepo {
    async fn get(&self, _guild_id: &str) -> Result<Option<IaConfig>, DomainError> {
        Ok(self.stored.lock().unwrap().clone())
    }
    async fn save(&self, config: &IaConfig) -> Result<IaConfig, DomainError> {
        *self.stored.lock().unwrap() = Some(config.clone());
        Ok(config.clone())
    }
}

fn build_app(repo: MockIaConfigRepo) -> axum::Router {
    router::build_for_test(build_test_state_ia_config(Arc::new(repo)))
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

async fn put_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("PUT").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

// ══════════════════════════════════════════════════════════
// GET /ia-config/{guild_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_returns_default_when_nothing_stored() {
    let app = build_app(MockIaConfigRepo::new());
    let (status, json) = get(app, "/api/ia-config/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["guild_id"], "111111111111111111");
    // Les defaults contiennent text_enabled et vision_enabled
    assert!(json.get("text_enabled").is_some());
    assert!(json.get("vision_enabled").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_returns_stored_config() {
    let c = IaConfig::new_normalized(
        "111111111111111111".into(),
        true, 0.75, false, 0.5, 0.2,
        "natural".into(), 10, 500,
    );
    let app = build_app(MockIaConfigRepo::new().with(c));
    let (status, json) = get(app, "/api/ia-config/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["text_enabled"], true);
    assert_eq!(json["vision_enabled"], false);
    assert_eq!(json["text_threshold"], 0.75);
}

// ══════════════════════════════════════════════════════════
// PUT /ia-config/{guild_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_saves_and_returns_normalized() {
    let app = build_app(MockIaConfigRepo::new());
    let body = serde_json::json!({
        "text_enabled": true,
        "text_threshold": 0.8,
        "vision_enabled": false,
        "vision_threshold": 0.5,
        "context_dampening": 0.2,
        "context_format": "natural",
        "context_max_messages": 10,
        "context_max_chars": 500
    });
    let (status, json) = put_json(app, "/api/ia-config/111111111111111111", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["guild_id"], "111111111111111111");
    assert_eq!(json["text_enabled"], true);
    assert_eq!(json["text_threshold"], 0.8);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_clamps_out_of_range_threshold() {
    let app = build_app(MockIaConfigRepo::new());
    let body = serde_json::json!({
        "text_enabled": true,
        "text_threshold": 99.0,
        "vision_enabled": false,
        "vision_threshold": -5.0,
        "context_dampening": 50.0,
        "context_format": "natural",
        "context_max_messages": 10,
        "context_max_chars": 500
    });
    let (status, json) = put_json(app, "/api/ia-config/111111111111111111", body).await;
    assert_eq!(status, StatusCode::OK);
    let t = json["text_threshold"].as_f64().unwrap();
    let v = json["vision_threshold"].as_f64().unwrap();
    assert!((0.0..=1.0).contains(&t), "text_threshold clamp: {t}");
    assert!((0.0..=1.0).contains(&v), "vision_threshold clamp: {v}");
}
