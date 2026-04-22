//! Tests d'integration HTTP pour GET /api/guilds/{guild_id}/channels.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_text_channels_returns_empty_array_from_mock() {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.discord_api = Arc::new(test_helpers::MockDiscordApi::new());
    let app = router::build_for_test(state);
    let req = Request::builder()
        .method("GET")
        .uri("/api/guilds/111111111111111111/channels")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&b).unwrap();
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_text_channels_second_call_hits_cache() {
    // Deux appels consecutifs : le second doit lire le cache Redis si pose.
    // On verifie surtout que le code path cache ne panique pas.
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.discord_api = Arc::new(test_helpers::MockDiscordApi::new());
    let app = router::build_for_test(state);

    for _ in 0..2 {
        let req = Request::builder()
            .method("GET")
            .uri("/api/guilds/222222222222222222/channels")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
