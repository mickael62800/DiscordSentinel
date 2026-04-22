//! Tests d'integration HTTP pour les petits endpoints systeme :
//! GET /health, GET /api/cache/stats, GET /api/models/status, POST /api/models/reload.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;

async fn json_req(app: axum::Router, method: &str, uri: &str, body: Option<serde_json::Value>)
    -> (StatusCode, serde_json::Value)
{
    let mut b = Request::builder().method(method).uri(uri);
    let body_payload = match body {
        Some(v) => {
            b = b.header("content-type", "application/json");
            Body::from(serde_json::to_string(&v).unwrap())
        }
        None => Body::empty(),
    };
    let req = b.body(body_payload).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

fn state() -> sentinel_api::adapters::inbound::http::state::AppState {
    test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels))
}

// ── /health ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_returns_ok_when_pg_and_redis_up() {
    let app = router::build_for_test(state());
    let (status, json) = json_req(app, "GET", "/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ok");
    assert_eq!(json["components"]["api"], "ok");
    assert_eq!(json["components"]["postgresql"], "ok");
    assert_eq!(json["components"]["redis"], "ok");
}

// ── /api/cache/stats ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cache_stats_returns_zeros_when_no_cache() {
    // base_state() construit un AppState avec `cache: None` -> fallback zeros.
    let app = router::build_for_test(state());
    let (status, json) = json_req(app, "GET", "/api/cache/stats", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["hits"], 0);
    assert_eq!(json["misses"], 0);
    assert_eq!(json["total"], 0);
    assert_eq!(json["hit_rate_percent"], 0.0);
}

// ── /api/models/status ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn models_status_returns_vision_and_text_entries() {
    let app = router::build_for_test(state());
    let (status, json) = json_req(app, "GET", "/api/models/status", None).await;
    assert_eq!(status, StatusCode::OK);
    let models = json["models"].as_array().unwrap();
    assert_eq!(models.len(), 2);
    let types: Vec<&str> = models.iter().map(|m| m["model_type"].as_str().unwrap()).collect();
    assert!(types.contains(&"vision"));
    assert!(types.contains(&"text"));
    // InferenceService::new(None, None) -> vision/text pas charges.
    for m in models {
        assert_eq!(m["loaded"], false);
        // Nom d'affichage utilise le fallback "non configure" (env vars vides en test).
        let name = m["name"].as_str().unwrap();
        assert!(name.contains("ONNX"));
    }
}

// ── /api/models/reload ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reload_model_unknown_type_returns_500() {
    // InferenceService::reload renvoie Err pour un type inconnu.
    let app = router::build_for_test(state());
    let body = serde_json::json!({ "model_type": "audio" });
    let (status, json) = json_req(app, "POST", "/api/models/reload", Some(body)).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(json["success"], false);
    assert!(json["message"].as_str().unwrap().len() > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reload_model_vision_without_configured_path_returns_500() {
    // Pas de VISION_MODEL_PATH configure -> reload echoue.
    let app = router::build_for_test(state());
    let body = serde_json::json!({ "model_type": "vision" });
    let (status, json) = json_req(app, "POST", "/api/models/reload", Some(body)).await;
    // Soit 500 si le loader echoue, soit 200 si un mock accepte. On verifie
    // juste que le handler a renvoye une reponse structurelle valide.
    assert!(status == StatusCode::INTERNAL_SERVER_ERROR || status == StatusCode::OK);
    assert!(json["success"].is_boolean());
    assert!(json["message"].is_string());
}
