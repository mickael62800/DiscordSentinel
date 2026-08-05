//! Tests d'integration HTTP pour GET/PUT /api/welcome/{guild_id}.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::ports::outbound::community::welcome_config_repository::WelcomeConfigData;
use sentinel_api::ports::outbound::community::welcome_config_repository::WelcomeConfigRepository;
use sentinel_core::domain::errors::DomainError;
use test_helpers::build_test_state_welcome;

#[derive(Default)]
struct MockWelcomeRepo {
    data: Mutex<Option<WelcomeConfigData>>,
}

fn default_data(guild_id: &str) -> WelcomeConfigData {
    WelcomeConfigData {
        guild_id: guild_id.into(),
        welcome_enabled: false,
        welcome_channel_id: None,
        welcome_message: "Welcome!".into(),
        welcome_embed_color: "#5865F2".into(),
        welcome_dm_enabled: false,
        welcome_dm_message: String::new(),
        leave_enabled: false,
        leave_channel_id: None,
        leave_message: String::new(),
        rules_enabled: false,
        rules_channel_id: None,
        rules_message: String::new(),
        rules_role_id: None,
        rules_button_label: "Accepter".into(),
        age_check_enabled: false,
        age_minimum: 0,
        unverified_role_id: None,
        age_modal_question: String::new(),
        age_ban_message: String::new(),
        age_min: 5,
        age_max: 120,
        age_ban_days_per_year: 365,
        age_ban_log_channel_id: None,
        leave_embed_color: "e74c3c".into(),
        rules_embed_color: "5865f2".into(),
        counter_enabled: false,
        counter_channel_id: None,
        counter_format: String::new(),
        voice_counter_enabled: false,
        voice_counter_channel_id: None,
        voice_counter_format: String::new(),
        anniversary_enabled: false,
        anniversary_channel_id: None,
        anniversary_message: String::new(),
        rejoin_message: String::new(),
        welcome_title: String::new(),
        welcome_image_url: String::new(),
        welcome_footer_text: String::new(),
        rejoin_title: String::new(),
        rejoin_image_url: String::new(),
        rejoin_footer_text: String::new(),
        leave_title: String::new(),
        leave_image_url: String::new(),
        leave_footer_text: String::new(),
        anniversary_title: String::new(),
        anniversary_image_url: String::new(),
        anniversary_footer_text: String::new(),
    }
}

#[async_trait]
impl WelcomeConfigRepository for MockWelcomeRepo {
    async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfigData, DomainError> {
        Ok(self
            .data
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| default_data(guild_id)))
    }
    async fn save_config(
        &self,
        _guild_id: &str,
        data: &WelcomeConfigData,
    ) -> Result<WelcomeConfigData, DomainError> {
        *self.data.lock().unwrap() = Some(data.clone());
        Ok(data.clone())
    }
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

async fn put_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("PUT")
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

fn build_app() -> (axum::Router, Arc<MockWelcomeRepo>) {
    let repo = Arc::new(MockWelcomeRepo::default());
    let app = router::build_for_test(build_test_state_welcome(repo.clone()));
    (app, repo)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_config_returns_default_when_empty() {
    let (app, _repo) = build_app();
    let (status, json) = get(app, "/api/welcome/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["guild_id"], "111111111111111111");
    assert_eq!(json["welcome_enabled"], false);
    assert_eq!(json["welcome_message"], "Welcome!");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_merges_partial_fields() {
    let (app, repo) = build_app();
    let body = serde_json::json!({
        "welcome_enabled": true,
        "welcome_channel_id": "c1",
        "welcome_message": "Hello {user}!"
    });
    let (status, json) = put_json(app, "/api/welcome/111111111111111111", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["welcome_enabled"], true);
    assert_eq!(json["welcome_channel_id"], "c1");
    assert_eq!(json["welcome_message"], "Hello {user}!");
    // Champs non touches : preservent leur default
    assert_eq!(json["welcome_embed_color"], "#5865F2");
    assert!(repo.data.lock().unwrap().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_then_get_returns_saved_data() {
    let (app1, repo) = build_app();
    let body = serde_json::json!({
        "leave_enabled": true,
        "leave_message": "Bye"
    });
    let (status, _) = put_json(app1, "/api/welcome/111111111111111111", body).await;
    assert_eq!(status, StatusCode::OK);

    // Rebuild app reusing the same repo to verify persistence.
    let app2 = router::build_for_test(build_test_state_welcome(repo.clone()));
    let (status, json) = get(app2, "/api/welcome/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["leave_enabled"], true);
    assert_eq!(json["leave_message"], "Bye");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_empty_body_keeps_defaults() {
    let (app, _repo) = build_app();
    let (status, json) = put_json(
        app,
        "/api/welcome/111111111111111111",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["welcome_enabled"], false);
    assert_eq!(json["welcome_message"], "Welcome!");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn put_full_body_updates_every_mergeable_field() {
    // Ce test couvre toutes les branches `if let Some(v) = dto.xxx` de save_config.
    let (app, repo) = build_app();
    let body = serde_json::json!({
        "welcome_enabled": true,
        "welcome_channel_id": "w-ch",
        "welcome_message": "Hi",
        "welcome_embed_color": "#FF0000",
        "welcome_dm_enabled": true,
        "welcome_dm_message": "DM",
        "leave_enabled": true,
        "leave_channel_id": "l-ch",
        "leave_message": "Bye",
        "rules_enabled": true,
        "rules_channel_id": "r-ch",
        "rules_message": "Rules",
        "rules_role_id": "role-1",
        "rules_button_label": "OK",
        "counter_enabled": true,
        "counter_channel_id": "c-ch",
        "counter_format": "{count}",
        "anniversary_enabled": true,
        "anniversary_channel_id": "a-ch",
        "anniversary_message": "Happy",
        "rejoin_message": "Welcome back"
    });
    let (status, json) = put_json(app, "/api/welcome/111111111111111111", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["welcome_channel_id"], "w-ch");
    assert_eq!(json["welcome_dm_message"], "DM");
    assert_eq!(json["leave_channel_id"], "l-ch");
    assert_eq!(json["rules_role_id"], "role-1");
    assert_eq!(json["rules_button_label"], "OK");
    assert_eq!(json["counter_channel_id"], "c-ch");
    assert_eq!(json["counter_format"], "{count}");
    assert_eq!(json["anniversary_enabled"], true);
    assert_eq!(json["anniversary_message"], "Happy");
    assert_eq!(json["rejoin_message"], "Welcome back");

    let saved = repo.data.lock().unwrap().clone().unwrap();
    assert_eq!(saved.welcome_channel_id.as_deref(), Some("w-ch"));
    assert_eq!(saved.leave_channel_id.as_deref(), Some("l-ch"));
    assert_eq!(saved.rules_role_id.as_deref(), Some("role-1"));
    assert_eq!(saved.counter_channel_id.as_deref(), Some("c-ch"));
    assert_eq!(saved.anniversary_channel_id.as_deref(), Some("a-ch"));
}
