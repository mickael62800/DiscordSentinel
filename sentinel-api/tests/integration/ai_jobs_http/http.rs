//! Tests d'integration HTTP pour POST /api/ai/jobs + GET /api/ai/jobs/{id}.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;

async fn json_req(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut b = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(v) => {
            b = b.header("content-type", "application/json");
            Body::from(serde_json::to_string(&v).unwrap())
        }
        None => Body::empty(),
    };
    let req = b.body(body).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

fn state() -> sentinel_api::adapters::inbound::http::state::AppState {
    test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_ai_job_rejects_invalid_job_type() {
    let app = router::build_for_test(state());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "job_type": "unknown",
        "input_payload": {}
    });
    let (status, json) = json_req(app, "POST", "/api/ai/jobs", Some(body)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("job_type"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_ai_job_rejects_empty_guild_id() {
    let app = router::build_for_test(state());
    let body = serde_json::json!({
        "guild_id": "",
        "job_type": "analyze_text",
        "input_payload": {}
    });
    let (status, json) = json_req(app, "POST", "/api/ai/jobs", Some(body)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("guild_id"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_ai_job_analyze_text_returns_202() {
    let app = router::build_for_test(state());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "job_type": "analyze_text",
        "input_payload": { "text": "hello" }
    });
    let (status, json) = json_req(app, "POST", "/api/ai/jobs", Some(body)).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(json["status"], "pending");
    assert!(uuid::Uuid::parse_str(json["job_id"].as_str().unwrap()).is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_ai_job_analyze_image_returns_202() {
    let app = router::build_for_test(state());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "job_type": "analyze_image",
        "input_payload": { "url": "https://x/y.png" }
    });
    let (status, _) = json_req(app, "POST", "/api/ai/jobs", Some(body)).await;
    assert_eq!(status, StatusCode::ACCEPTED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_ai_job_invalid_uuid_422() {
    let app = router::build_for_test(state());
    let (status, json) = json_req(app, "GET", "/api/ai/jobs/not-a-uuid", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("invalide"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_ai_job_unknown_uuid_404() {
    let app = router::build_for_test(state());
    let id = uuid::Uuid::new_v4();
    let (status, _) = json_req(app, "GET", &format!("/api/ai/jobs/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_then_get_ai_job_roundtrip() {
    let app = router::build_for_test(state());
    let body = serde_json::json!({
        "guild_id": "999999999999999999",
        "job_type": "analyze_text",
        "input_payload": { "text": "roundtrip" }
    });
    let (s1, j1) = json_req(app.clone(), "POST", "/api/ai/jobs", Some(body)).await;
    assert_eq!(s1, StatusCode::ACCEPTED);
    let job_id = j1["job_id"].as_str().unwrap();
    let (s2, j2) = json_req(app, "GET", &format!("/api/ai/jobs/{job_id}"), None).await;
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(j2["job_type"], "analyze_text");
    assert_eq!(j2["guild_id"], "999999999999999999");
}
