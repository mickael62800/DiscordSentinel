//! Tests d'integration HTTP pour les endpoints rules.

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
use sentinel_api::ports::inbound::moderation::manage_rules::CreateRuleCommand;
use sentinel_api::ports::inbound::moderation::manage_rules::ManageRulesUseCase;
use sentinel_core::domain::entities::system::rule::Rule;
use sentinel_core::domain::enums::moderation::flag_type::FlagType;
use sentinel_core::domain::errors::DomainError;
use test_helpers::build_test_state_rules;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockRulesUC {
    rules: Mutex<Vec<Rule>>,
    deleted: Mutex<Vec<(String, Uuid)>>,
}

impl MockRulesUC {
    fn new() -> Self {
        Self::default()
    }

    fn with_rule(self, r: Rule) -> Self {
        self.rules.lock().unwrap().push(r);
        self
    }
}

#[async_trait]
impl ManageRulesUseCase for MockRulesUC {
    async fn get_rules(&self, guild_id: &str) -> Result<Vec<Rule>, DomainError> {
        Ok(self
            .rules
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.guild_id == guild_id)
            .cloned()
            .collect())
    }
    async fn get_all_rules(&self) -> Result<Vec<Rule>, DomainError> {
        Ok(self.rules.lock().unwrap().clone())
    }
    async fn toggle_rule(&self, _: Uuid, _: bool) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn create_or_update_rule(&self, cmd: CreateRuleCommand) -> Result<Rule, DomainError> {
        let now = Utc::now();
        let rule = Rule {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            flag_type: cmd.flag_type,
            weight: cmd.weight,
            threshold_warn: cmd.threshold_warn,
            threshold_delete: cmd.threshold_delete,
            threshold_mute: cmd.threshold_mute,
            threshold_ban: cmd.threshold_ban,
            enabled: cmd.enabled,
            created_at: now,
            updated_at: now,
        };
        self.rules.lock().unwrap().push(rule.clone());
        Ok(rule)
    }
    async fn delete_rule(&self, guild_id: &str, rule_id: Uuid) -> Result<(), DomainError> {
        self.deleted
            .lock()
            .unwrap()
            .push((guild_id.into(), rule_id));
        self.rules
            .lock()
            .unwrap()
            .retain(|r| !(r.guild_id == guild_id && r.id == rule_id));
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_app(uc: MockRulesUC) -> axum::Router {
    router::build_for_test(build_test_state_rules(Arc::new(uc)))
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

async fn delete(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("DELETE")
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

fn sample_rule(guild_id: &str, flag_type: FlagType) -> Rule {
    let now = Utc::now();
    Rule {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        flag_type,
        weight: 3.0,
        threshold_warn: 2.0,
        threshold_delete: 4.0,
        threshold_mute: 6.0,
        threshold_ban: 9.0,
        enabled: true,
        created_at: now,
        updated_at: now,
    }
}

// ══════════════════════════════════════════════════════════
// GET /rules/{guild_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_rules_empty() {
    let app = build_app(MockRulesUC::new());
    let (status, json) = get(app, "/rules/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_rules_returns_only_guild_scoped() {
    let uc = MockRulesUC::new()
        .with_rule(sample_rule("111111111111111111", FlagType::Spam))
        .with_rule(sample_rule("111111111111111111", FlagType::Insult))
        .with_rule(sample_rule("222222222222222222", FlagType::Spam));
    let app = build_app(uc);
    let (status, json) = get(app, "/rules/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 2);
}

// ══════════════════════════════════════════════════════════
// POST /rules
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_rule_success() {
    let app = build_app(MockRulesUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "flag_type": "spam",
        "weight": 3.0,
        "threshold_warn": 2.0,
        "threshold_delete": 4.0,
        "threshold_mute": 6.0,
        "threshold_ban": 9.0,
        "enabled": true
    });
    let (status, json) = post_json(app, "/rules", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["guild_id"], "111111111111111111");
    assert_eq!(json["flag_type"], "spam");
    assert_eq!(json["weight"], 3.0);
    assert!(json["id"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_rule_defaults_enabled_true() {
    // `enabled` absent du body → default true via serde.
    let app = build_app(MockRulesUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "flag_type": "spam",
        "weight": 3.0,
        "threshold_warn": 2.0,
        "threshold_delete": 4.0,
        "threshold_mute": 6.0,
        "threshold_ban": 9.0
    });
    let (status, json) = post_json(app, "/rules", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["enabled"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_rule_missing_required_field_400() {
    // weight manquant → serde renvoie 400 Bad Request
    let app = build_app(MockRulesUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "flag_type": "spam"
    });
    let (status, _) = post_json(app, "/rules", body).await;
    assert!(status.is_client_error(), "expected 4xx, got {status}");
}

// ══════════════════════════════════════════════════════════
// DELETE /rules/{guild_id}/{rule_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_rule_success_no_rbac_header() {
    let rule = sample_rule("111111111111111111", FlagType::Link);
    let rule_id = rule.id;
    let app = build_app(MockRulesUC::new().with_rule(rule));
    let (status, json) = delete(app, &format!("/rules/111111111111111111/{rule_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["deleted"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_rule_invalid_uuid_422() {
    let app = build_app(MockRulesUC::new());
    let (status, _) = delete(app, "/rules/111111111111111111/not-a-uuid").await;
    assert!(status.is_client_error());
}
