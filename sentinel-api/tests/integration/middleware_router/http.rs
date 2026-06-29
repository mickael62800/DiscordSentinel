//! Tests E2E HTTP qui exercent la stack complete :
//! router::build_for_test -> auth_middleware -> handlers.
//!
//! Couvre :
//! - auth_middleware : passthrough quand api_key vide, 401 sans Bearer quand configure
//! - OAuth /authorize / /callback : 503 quand OAuth non configure
//! - 404 sur routes inconnues
//! - Routes publiques (health) ne passent pas par auth_middleware

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use axum::body::Body;
use axum::http::header;
use axum::http::Request;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::adapters::inbound::http::state::AppState;

fn base() -> AppState {
    test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels))
}

fn with_api_key(key: &str) -> AppState {
    let mut s = base();
    s.api_key = key.into();
    s
}

async fn do_req(
    app: axum::Router,
    method: &str,
    uri: &str,
    headers: &[(&str, &str)],
) -> (StatusCode, Vec<u8>) {
    let mut b = Request::builder().method(method).uri(uri);
    for (k, v) in headers {
        b = b.header(*k, *v);
    }
    let req = b.body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (s, bytes.to_vec())
}

// ── auth_middleware : passthrough vs 401 ─────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auth_passes_when_api_key_empty() {
    // api_key vide → auth_middleware passe tout le monde (dev mode)
    let app = router::build_for_test(base());
    let (status, _) = do_req(app, "GET", "/api/cache/stats", &[]).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auth_rejects_missing_bearer_when_api_key_set() {
    let app = router::build_for_test(with_api_key("s3cret-key-test-1234"));
    let (status, _) = do_req(app, "GET", "/api/cache/stats", &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auth_rejects_wrong_bearer() {
    let app = router::build_for_test(with_api_key("correct-key"));
    let (status, _) = do_req(
        app,
        "GET",
        "/api/cache/stats",
        &[("authorization", "Bearer wrong-key")],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auth_accepts_correct_bearer() {
    let key = "correct-key-1234567890";
    let app = router::build_for_test(with_api_key(key));
    let (status, _) = do_req(
        app,
        "GET",
        "/api/cache/stats",
        &[("authorization", &format!("Bearer {key}"))],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auth_rejects_malformed_authorization() {
    let app = router::build_for_test(with_api_key("key"));
    // Pas "Bearer " prefix → 401
    let (status, _) = do_req(
        app,
        "GET",
        "/api/cache/stats",
        &[("authorization", "Basic dXNlcjpwYXNz")],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ── Routes publiques (health, oauth) : pas d'auth ─────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_is_public_no_auth_required() {
    // Meme avec api_key configure, /health repond sans Bearer
    let app = router::build_for_test(with_api_key("k"));
    let (status, _) = do_req(app, "GET", "/health", &[]).await;
    // OK ou 503 selon l'etat DB/Redis — on verifie juste que ce n'est PAS 401
    assert_ne!(status, StatusCode::UNAUTHORIZED);
    assert!(status == StatusCode::OK || status == StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oauth_authorize_returns_503_when_not_configured() {
    // base_state() laisse discord_oauth_* vides → 503
    let app = router::build_for_test(base());
    let (status, _) = do_req(app, "GET", "/auth/discord/authorize", &[]).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oauth_callback_without_code_redirects_or_errors() {
    let app = router::build_for_test(base());
    let (status, _) = do_req(app, "GET", "/auth/discord/callback", &[]).await;
    // Sans query params valides, on attend soit 302 (redirect d'erreur vers front)
    // soit 400 selon le flow. On verifie juste que ce n'est pas 200/404.
    assert!(
        status == StatusCode::FOUND
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::SERVICE_UNAVAILABLE
            || status == StatusCode::INTERNAL_SERVER_ERROR,
        "unexpected status: {status}"
    );
}

// ── 404 sur routes inconnues ──────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_route_returns_404() {
    let app = router::build_for_test(base());
    let (status, _) = do_req(app, "GET", "/api/route-qui-n-existe-pas", &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_top_level_path_returns_404() {
    let app = router::build_for_test(base());
    let (status, _) = do_req(app, "GET", "/foo/bar", &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wrong_method_on_existing_route_returns_method_not_allowed_or_404() {
    // /health est GET only. DELETE doit retourner 405 ou 404.
    let app = router::build_for_test(base());
    let (status, _) = do_req(app, "DELETE", "/health", &[]).await;
    assert!(
        status == StatusCode::METHOD_NOT_ALLOWED || status == StatusCode::NOT_FOUND,
        "unexpected: {status}"
    );
}

// ── Content-Type / auth edge cases ────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auth_rejects_bearer_without_space() {
    let app = router::build_for_test(with_api_key("k"));
    let (status, _) = do_req(
        app,
        "GET",
        "/api/cache/stats",
        &[("authorization", "Bearer")],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auth_accepts_key_short_but_matching() {
    // API key short mais egale a ce qui est configure → passe quand meme
    let app = router::build_for_test(with_api_key("abc"));
    let (status, _) = do_req(
        app,
        "GET",
        "/api/cache/stats",
        &[("authorization", "Bearer abc")],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

// ── Health body structure ─────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_body_has_expected_keys() {
    let app = router::build_for_test(base());
    let (_, body) = do_req(app, "GET", "/health", &[]).await;
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["status"].is_string());
    assert!(v["components"].is_object());
    assert!(v["components"]["api"].is_string());
    assert!(v["components"]["postgresql"].is_string());
    assert!(v["components"]["redis"].is_string());
}

// ── Header insensible à la casse (RFC 7230) ────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auth_header_case_insensitive() {
    let key = "mykey";
    let app = router::build_for_test(with_api_key(key));
    // Axum lower-case les headers, "Authorization" vs "authorization" equivalent
    let (status, _) = do_req(
        app,
        "GET",
        "/api/cache/stats",
        &[(header::AUTHORIZATION.as_str(), &format!("Bearer {key}"))],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}
