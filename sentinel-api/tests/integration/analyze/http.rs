//! Tests d'integration HTTP pour POST /analyze.

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
use sentinel_api::ports::inbound::ai::analyze_message::AnalyzeMessageCommand;
use sentinel_api::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use sentinel_core::domain::entities::ai::message_analysis::MessageAnalysis;
use sentinel_core::domain::enums::moderation::action::Action;
use sentinel_core::domain::errors::DomainError;
use test_helpers::build_test_state_analyze;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

struct MockAnalyzeUC {
    response: MessageAnalysis,
    calls: Mutex<Vec<AnalyzeMessageCommand>>,
}

impl MockAnalyzeUC {
    fn returning(action: Action, reason: &str) -> Self {
        Self {
            response: MessageAnalysis {
                action,
                reason: reason.into(),
                score: 0.7,
                duration: None,
                route: sentinel_core::domain::services::moderation::automod_routing::Routing::None,
                severe: false,
                auto_delete_link: false,
            },
            calls: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl AnalyzeMessageUseCase for MockAnalyzeUC {
    async fn analyze(&self, cmd: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError> {
        self.calls.lock().unwrap().push(cmd);
        Ok(self.response.clone())
    }
    async fn evaluate_flood(
        &self,
        _: &str,
        _: i32,
    ) -> Result<sentinel_core::ports::inbound::ai::analyze_message::FloodDecision, DomainError>
    {
        unimplemented!()
    }
    async fn evaluate_attachments(
        &self,
        _: &str,
        _: Vec<String>,
    ) -> Result<sentinel_core::ports::inbound::ai::analyze_message::AttachmentDecision, DomainError>
    {
        unimplemented!()
    }
}

fn build_app(uc: MockAnalyzeUC) -> axum::Router {
    router::build_for_test(build_test_state_analyze(Arc::new(uc)))
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

fn analyze_body(guild_id: &str, content: &str) -> serde_json::Value {
    serde_json::json!({
        "guild_id": guild_id,
        "channel_id": "555555555555555555",
        "user_id": "444444444444444444",
        "username": "alice",
        "content": content,
        "flags": {"spam": false, "insult": false, "link": false, "phishing": false},
        "metadata": {"message_id": "666666666666666666", "timestamp": "2024-01-01T00:00:00Z"},
        "context_messages": []
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_returns_action_and_reason() {
    let app = build_app(MockAnalyzeUC::returning(Action::Warn, "spam detecte"));
    let (status, json) =
        post_json(app, "/analyze", analyze_body("111111111111111111", "hello")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["action"], "warn");
    assert_eq!(json["reason"], "spam detecte");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_omits_empty_reason_in_json() {
    let app = build_app(MockAnalyzeUC::returning(Action::None, ""));
    let (status, json) = post_json(app, "/analyze", analyze_body("111111111111111111", "hi")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["action"], "none");
    assert!(json.get("reason").is_none());
    assert!(json.get("duration").is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_validates_guild_id() {
    let app = build_app(MockAnalyzeUC::returning(Action::None, ""));
    let (status, _) = post_json(app, "/analyze", analyze_body("not-a-snowflake", "hi")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_validates_user_id() {
    let app = build_app(MockAnalyzeUC::returning(Action::None, ""));
    let mut body = analyze_body("111111111111111111", "hi");
    body["user_id"] = serde_json::json!("abc");
    let (status, _) = post_json(app, "/analyze", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_truncates_content_over_2500_chars() {
    // On ne peut pas inspecter le command depuis le handler, mais on verifie
    // au moins que la requete passe (truncation silencieuse cote DTO).
    let uc = Arc::new(MockAnalyzeUC::returning(Action::None, ""));
    let state = build_test_state_analyze(uc.clone());
    let app = router::build_for_test(state);
    let mut body = analyze_body("111111111111111111", "hi");
    body["content"] = serde_json::Value::String("a".repeat(5000));
    let (status, _) = post_json(app, "/analyze", body).await;
    assert_eq!(status, StatusCode::OK);
    let calls = uc.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].content.len(), 2500);
}
