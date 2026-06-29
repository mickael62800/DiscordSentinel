//! Tests d'integration HTTP pour les endpoints systeme (models, cache).

#[path = "../../test_helpers.rs"]
mod test_helpers;

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;

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
// GET /api/models/status
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn models_status_returns_two_entries() {
    // StubAnalyzeImage / base_state utilisent un InferenceService::new(None, None)
    // → vision et text pas charges.
    let state =
        test_helpers::build_test_state(std::sync::Arc::new(test_helpers::StubVoiceChannels));
    let app = router::build_for_test(state);
    let (status, json) = get(app, "/api/models/status").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json["models"].as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["model_type"], "vision");
    assert_eq!(arr[1]["model_type"], "text");
    assert_eq!(arr[0]["loaded"], false);
    assert_eq!(arr[1]["loaded"], false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn models_status_name_indicates_non_configure_when_env_empty() {
    // SAFETY : test mono-process, remove_var avant le handler.
    unsafe {
        std::env::remove_var("VISION_MODEL_PATH");
        std::env::remove_var("TEXT_MODEL_PATH");
    }
    let state =
        test_helpers::build_test_state(std::sync::Arc::new(test_helpers::StubVoiceChannels));
    let app = router::build_for_test(state);
    let (status, json) = get(app, "/api/models/status").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json["models"].as_array().unwrap();
    assert!(arr[0]["name"].as_str().unwrap().contains("non configure"));
    assert!(arr[1]["name"].as_str().unwrap().contains("non configure"));
}

// ══════════════════════════════════════════════════════════
// POST /api/models/reload
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reload_unknown_type_returns_500_with_message() {
    let state =
        test_helpers::build_test_state(std::sync::Arc::new(test_helpers::StubVoiceChannels));
    let app = router::build_for_test(state);
    let body = serde_json::json!({"model_type": "something-unknown"});
    let (status, json) = post_json(app, "/api/models/reload", body).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(json["success"], false);
    assert!(json["message"].as_str().unwrap().contains("inconnu"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reload_text_without_env_returns_500() {
    unsafe {
        std::env::remove_var("TEXT_MODEL_PATH");
    }
    let state =
        test_helpers::build_test_state(std::sync::Arc::new(test_helpers::StubVoiceChannels));
    let app = router::build_for_test(state);
    let body = serde_json::json!({"model_type": "text"});
    let (status, json) = post_json(app, "/api/models/reload", body).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(json["success"], false);
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("TEXT_MODEL_PATH"));
}

// ══════════════════════════════════════════════════════════
// GET /api/cache/stats
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cache_stats_returns_zeros_when_no_cache() {
    // base_state initialise state.cache = None
    let state =
        test_helpers::build_test_state(std::sync::Arc::new(test_helpers::StubVoiceChannels));
    let app = router::build_for_test(state);
    let (status, json) = get(app, "/api/cache/stats").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["hits"], 0);
    assert_eq!(json["misses"], 0);
    assert_eq!(json["total"], 0);
    assert_eq!(json["hit_rate_percent"], 0.0);
}
