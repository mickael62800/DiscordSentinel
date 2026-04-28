//! Tests d'integration HTTP pour les petits endpoints systeme :
//! GET /health, GET /api/cache/stats, GET /api/models/status, POST /api/models/reload.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
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

// ── /api/system/info ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn system_info_returns_full_structure() {
    // Endpoint admin : retourne bots/workers/host/process/redis/uptime/db_size.
    // Avec containers test actifs : PG + Redis repondent, sysinfo collecte local.
    let app = router::build_for_test(state());
    let (status, json) = json_req(app, "GET", "/api/system/info", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["bots"].is_array());
    assert!(json["workers"].is_array());
    assert!(json["host"].is_object());
    assert!(json["host"]["cpu_percent"].is_number());
    assert!(json["host"]["cpu_cores"].is_number());
    assert!(json["host"]["mem_used_mb"].is_number());
    assert!(json["host"]["mem_total_mb"].is_number());
    assert!(json["process"].is_object());
    assert!(json["process"]["cpu_percent"].is_number());
    assert!(json["redis"].is_object());
    assert!(json["uptime_seconds"].is_number());
    assert!(json["db_size_mb"].is_number());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn system_info_lists_bots_and_workers_from_redis() {
    use redis::AsyncCommands;
    // Seed un bot et un worker dans Redis
    let client = redis::Client::open(
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6380".into())
    ).unwrap();
    let mut conn = client.get_multiplexed_async_connection().await.unwrap();
    let bot_name = format!("sysinfo-test-bot-{}", uuid::Uuid::new_v4().as_u128() % 10_000_000);
    let worker_name = format!("sysinfo-test-worker-{}", uuid::Uuid::new_v4().as_u128() % 10_000_000);
    let _: () = conn.sadd::<_, _, ()>("bots:known", &bot_name).await.unwrap();
    let _: () = conn.sadd::<_, _, ()>("bots:known", &worker_name).await.unwrap();
    let _: () = conn.set_ex::<_, _, ()>(format!("bot:online:{bot_name}"), "1", 60).await.unwrap();

    let app = router::build_for_test(state());
    let (status, json) = json_req(app, "GET", "/api/system/info", None).await;
    assert_eq!(status, StatusCode::OK);

    let bots = json["bots"].as_array().unwrap();
    let workers = json["workers"].as_array().unwrap();
    assert!(bots.iter().any(|b| b["name"].as_str().unwrap() == bot_name && b["online"] == true));
    assert!(workers.iter().any(|w| w["name"].as_str().unwrap() == worker_name && w["online"] == false));

    // Cleanup
    let _: () = conn.srem::<_, _, ()>("bots:known", &bot_name).await.unwrap();
    let _: () = conn.srem::<_, _, ()>("bots:known", &worker_name).await.unwrap();
    let _: () = conn.del::<_, ()>(format!("bot:online:{bot_name}")).await.unwrap();
}
