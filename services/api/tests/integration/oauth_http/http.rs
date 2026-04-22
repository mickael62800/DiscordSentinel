//! Tests d'integration HTTP pour les endpoints OAuth Discord.
//!
//! Couvre les branches testables sans mock HTTP : validation config,
//! generation du state Redis, redirections d'erreur cote callback.
//! L'echange code->token (qui tape reqwest::post sur Discord) n'est pas
//! testable ici et reste couvert par des tests E2E manuels.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use redis::AsyncCommands;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;

fn configured_state(front_url: &str) -> sentinel_api::adapters::inbound::http::state::AppState {
    let mut s = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    s.discord_oauth_client_id = "test-client-id".into();
    s.discord_oauth_client_secret = "test-secret".into();
    s.discord_oauth_redirect_uri = "https://api.example/auth/discord/callback".into();
    s.web_front_url = front_url.into();
    s
}

async fn get_resp(app: axum::Router, uri: &str) -> axum::response::Response {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    app.oneshot(req).await.unwrap()
}

fn location(resp: &axum::response::Response) -> String {
    resp.headers().get(axum::http::header::LOCATION).unwrap()
        .to_str().unwrap().to_string()
}

// ── /auth/discord/authorize ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authorize_without_config_returns_503() {
    // base_state() laisse les OAuth fields vides.
    let state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    let app = router::build_for_test(state);
    let resp = get_resp(app, "/auth/discord/authorize").await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authorize_success_redirects_to_discord_with_state_in_redis() {
    let state = configured_state("https://front.example");
    let redis_client = state.redis_client.clone();
    let app = router::build_for_test(state);
    let resp = get_resp(app, "/auth/discord/authorize").await;
    assert_eq!(resp.status(), StatusCode::FOUND);
    let loc = location(&resp);
    assert!(loc.starts_with("https://discord.com/api/oauth2/authorize?"));
    assert!(loc.contains("client_id=test-client-id"));
    assert!(loc.contains("scope=identify%20guilds"));
    assert!(loc.contains("response_type=code"));
    assert!(loc.contains("prompt=none"));
    assert!(loc.contains("redirect_uri=https%3A%2F%2Fapi.example%2Fauth%2Fdiscord%2Fcallback"));

    // state=<uuid> extrait + verifie en Redis.
    let state_val = loc.split("state=").nth(1).unwrap().split('&').next().unwrap();
    let mut conn = redis_client.get_multiplexed_async_connection().await.unwrap();
    let key = format!("oauth:web:state:{}", state_val);
    let stored: Option<String> = conn.get(&key).await.unwrap();
    assert_eq!(stored.as_deref(), Some("1"));
    let ttl: i64 = redis::cmd("TTL").arg(&key).query_async(&mut conn).await.unwrap();
    assert!(ttl > 0 && ttl <= 600);
}

// ── /auth/discord/callback ── (erreurs + state)

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn callback_discord_error_redirects_with_description() {
    let state = configured_state("https://front.example");
    let app = router::build_for_test(state);
    let resp = get_resp(app,
        "/auth/discord/callback?error=access_denied&error_description=User%20refused").await;
    assert_eq!(resp.status(), StatusCode::FOUND);
    let loc = location(&resp);
    assert!(loc.starts_with("https://front.example/login?error="));
    assert!(loc.contains("User%20refused"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn callback_discord_error_without_description_uses_error_code() {
    let state = configured_state("https://front.example");
    let app = router::build_for_test(state);
    let resp = get_resp(app, "/auth/discord/callback?error=access_denied").await;
    let loc = location(&resp);
    assert!(loc.contains("error=access_denied"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn callback_missing_code_redirects_with_code_manquant() {
    let state = configured_state("https://front.example");
    let app = router::build_for_test(state);
    let resp = get_resp(app, "/auth/discord/callback?state=abc").await;
    let loc = location(&resp);
    assert!(loc.contains("error=code_manquant"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn callback_missing_state_redirects_with_state_manquant() {
    let state = configured_state("https://front.example");
    let app = router::build_for_test(state);
    let resp = get_resp(app, "/auth/discord/callback?code=xyz").await;
    let loc = location(&resp);
    assert!(loc.contains("error=state_manquant"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn callback_empty_code_treated_as_missing() {
    let state = configured_state("https://front.example");
    let app = router::build_for_test(state);
    let resp = get_resp(app, "/auth/discord/callback?code=&state=abc").await;
    let loc = location(&resp);
    assert!(loc.contains("error=code_manquant"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn callback_unknown_state_redirects_with_state_invalide() {
    let state = configured_state("https://front.example");
    let app = router::build_for_test(state);
    // state non present en Redis -> state_invalide.
    let resp = get_resp(app,
        "/auth/discord/callback?code=xyz&state=never-existed").await;
    let loc = location(&resp);
    assert!(loc.contains("error=state_invalide"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn callback_empty_web_front_falls_back_to_root() {
    // web_front_url vide -> redirections utilisent "/".
    let state = configured_state("");
    let app = router::build_for_test(state);
    let resp = get_resp(app, "/auth/discord/callback?code=xyz").await;
    let loc = location(&resp);
    // "/" + trim_end_matches('/') -> "" -> "/login?error=..."
    assert!(loc.starts_with("/login?error=") || loc.starts_with("login?error="),
            "loc attendue relative: {loc}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn callback_valid_state_is_consumed_one_shot() {
    // Verifie que le state est DEL apres consommation : un 2eme callback
    // avec le meme state doit repondre state_invalide. On ne peut pas aller
    // jusqu'au bout (reqwest vers Discord echoue), mais le DEL se produit
    // avant l'appel reqwest.
    let state = configured_state("https://front.example");
    let redis_client = state.redis_client.clone();
    let app = router::build_for_test(state);

    // Seed un state en Redis a la main.
    let mut conn = redis_client.get_multiplexed_async_connection().await.unwrap();
    let csrf = "test-csrf-one-shot";
    let key = format!("oauth:web:state:{}", csrf);
    conn.set_ex::<_, _, ()>(&key, "1", 600).await.unwrap();

    // 1er hit : passe le state check, mais echoue plus tard (reqwest Discord).
    let resp1 = get_resp(app.clone(),
        &format!("/auth/discord/callback?code=fake&state={csrf}")).await;
    let loc1 = location(&resp1);
    // state_invalide NE doit PAS apparaitre — on a passe le check.
    assert!(!loc1.contains("error=state_invalide"),
            "1er appel: state devait etre accepte, recu: {loc1}");

    // 2eme hit avec le meme state : Redis ne contient plus la cle.
    let resp2 = get_resp(app,
        &format!("/auth/discord/callback?code=fake&state={csrf}")).await;
    let loc2 = location(&resp2);
    assert!(loc2.contains("error=state_invalide"),
            "2eme appel: state devait etre consomme, recu: {loc2}");
}
