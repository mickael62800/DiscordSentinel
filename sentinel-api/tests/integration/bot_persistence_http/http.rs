//! Tests d'integration HTTP pour bot_persistence (name_history, streaks,
//! sla_tickets, sponsorships, temp_roles, pending_actions).

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::adapters::inbound::http::state::AppState;

fn base_state() -> AppState {
    test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels))
}

async fn post_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("POST").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null))
}

async fn patch_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("PATCH").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null))
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null))
}

async fn delete(app: axum::Router, uri: &str) -> StatusCode {
    let req = Request::builder().method("DELETE").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    resp.status()
}

async fn send_request(app: axum::Router, req: Request<Body>) -> (StatusCode, serde_json::Value) {
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null))
}

// ══════════════════════════════════════════════════════════
// POST /api/name-history
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_name_history_success() {
    let mut state = base_state();
    // Le stub AuditLogs retourne un vrai AuditLog (cf test_helpers).
    // On utilise le stub par defaut.
    // Mais le stub StubAuditLogs::create retourne unimplemented! par defaut
    // — sauf modifs. Le handler inspect_err().ok() absorbera l'erreur, donc
    // l'endpoint retourne 200 quand meme.
    let _ = &mut state;
    let app = router::build_for_test(base_state());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444",
        "old_name": "oldname",
        "new_name": "newname"
    });
    let (status, json) = post_json(app, "/api/name-history", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_name_history_invalid_guild_422() {
    let app = router::build_for_test(base_state());
    let body = serde_json::json!({
        "guild_id": "bad",
        "user_id": "444444444444444444",
        "old_name": "a",
        "new_name": "b"
    });
    let (status, _) = post_json(app, "/api/name-history", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// PATCH /api/levels/{guild}/{user}/streak
// (sqlx direct → utilise vraie DB)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_streak_success() {
    // Meme si la row n'existe pas, l'UPDATE ne fail pas, inspect_err absorbe.
    let app = router::build_for_test(base_state());
    let body = serde_json::json!({
        "streak_current": 5, "streak_best": 10,
        "streak_last_day": 42, "streak_last_year": 2024
    });
    let (status, _) = patch_json(app, "/api/levels/111111111111111111/444444444444444444/streak", body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_streak_invalid_guild_422() {
    let app = router::build_for_test(base_state());
    let body = serde_json::json!({
        "streak_current": 1, "streak_best": 1, "streak_last_day": 1, "streak_last_year": 2024
    });
    let (status, _) = patch_json(app, "/api/levels/bad/444444444444444444/streak", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// PATCH /api/tickets/{id}/sla
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_ticket_sla_invalid_uuid_422() {
    let app = router::build_for_test(base_state());
    let body = serde_json::json!({"satisfaction_rating": 5});
    let (status, json) = patch_json(app, "/api/tickets/not-a-uuid/sla", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("ticket id invalide"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_ticket_sla_valid_uuid_ok() {
    let app = router::build_for_test(base_state());
    let id = Uuid::new_v4();
    let body = serde_json::json!({
        "first_response_at": "2024-01-01T00:00:00Z",
        "resolved_at": "2024-01-02T00:00:00Z",
        "satisfaction_rating": 4
    });
    let (status, _) = patch_json(app, &format!("/api/tickets/{id}/sla"), body).await;
    assert_eq!(status, StatusCode::OK);
}

// ══════════════════════════════════════════════════════════
// Sponsorships
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_sponsorship_success_without_rbac() {
    // Pas de rbac header → pass-through
    let app = router::build_for_test(base_state());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "sponsor_id": "444444444444444444",
        "sponsored_id": "555555555555555555"
    });
    let (status, _) = post_json(app, "/api/sponsorships", body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_sponsorship_invalid_sponsor_422() {
    let app = router::build_for_test(base_state());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "sponsor_id": "bad",
        "sponsored_id": "555555555555555555"
    });
    let (status, _) = post_json(app, "/api/sponsorships", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_sponsorship_with_rbac_viewer_forbidden() {
    use sentinel_core::domain::enums::system::role::Role;
    let app = router::build_for_test(base_state());
    let req = test_helpers::request_with_rbac(
        "POST", "/api/sponsorships",
        "444444444444444444", Some(Role::Viewer),
        Some("111111111111111111".into()),
        Some(serde_json::json!({
            "guild_id": "111111111111111111",
            "sponsor_id": "444444444444444444",
            "sponsored_id": "555555555555555555"
        })),
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_sponsorships_empty() {
    let app = router::build_for_test(base_state());
    let (status, json) = get(app, "/api/sponsorships/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_sponsorships_invalid_guild_422() {
    let app = router::build_for_test(base_state());
    let (status, _) = get(app, "/api/sponsorships/bad").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// Temp Roles
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_temp_role_success() {
    let app = router::build_for_test(base_state());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444",
        "role_id": "555555555555555555",
        "expires_at": "2024-12-31T23:59:59Z"
    });
    let (status, _) = post_json(app, "/api/temp-roles", body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_temp_role_invalid_role_id_422() {
    let app = router::build_for_test(base_state());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444",
        "role_id": "bad",
        "expires_at": "2024-12-31T23:59:59Z"
    });
    let (status, _) = post_json(app, "/api/temp-roles", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_temp_roles_empty() {
    let app = router::build_for_test(base_state());
    let (status, json) = get(app, "/api/temp-roles/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_temp_role_success_without_rbac() {
    let app = router::build_for_test(base_state());
    let status = delete(app, "/api/temp-roles/111111111111111111/444444444444444444/555555555555555555").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_temp_role_invalid_role_id_422() {
    let app = router::build_for_test(base_state());
    let status = delete(app, "/api/temp-roles/111111111111111111/444444444444444444/bad").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_temp_role_with_rbac_viewer_forbidden() {
    use sentinel_core::domain::enums::system::role::Role;
    let app = router::build_for_test(base_state());
    let req = test_helpers::request_with_rbac(
        "DELETE", "/api/temp-roles/111111111111111111/444444444444444444/555555555555555555",
        "444444444444444444", Some(Role::Viewer), Some("111111111111111111".into()),
        None,
    );
    let (status, json) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(json["error"].as_str().unwrap().contains("moderator+"));
}

// ══════════════════════════════════════════════════════════
// Pending Moderation Actions
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_pending_action_success() {
    let app = router::build_for_test(base_state());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "moderator_id": "444444444444444444",
        "moderator_name": "Mod",
        "target_id": "555555555555555555",
        "target_name": "User",
        "action_type": "warn",
        "reason": "Test reason"
    });
    let (status, json) = post_json(app, "/api/moderation/pending", body).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["id"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_pending_action_invalid_guild_422() {
    let app = router::build_for_test(base_state());
    let body = serde_json::json!({
        "guild_id": "bad",
        "moderator_id": "444444444444444444",
        "moderator_name": "Mod",
        "target_id": "555555555555555555",
        "target_name": "User",
        "action_type": "warn",
        "reason": "r"
    });
    let (status, _) = post_json(app, "/api/moderation/pending", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_pending_actions_empty() {
    let app = router::build_for_test(base_state());
    let (status, json) = get(app, "/api/moderation/pending/guild/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_pending_action_success_without_rbac() {
    let app = router::build_for_test(base_state());
    let id = Uuid::new_v4();
    let body = serde_json::json!({"status": "approved", "reviewed_by": "444444444444444444"});
    let (status, _) = patch_json(app, &format!("/api/moderation/pending/{id}/resolve"), body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_pending_action_invalid_id_422() {
    let app = router::build_for_test(base_state());
    let body = serde_json::json!({"status": "approved", "reviewed_by": "444444444444444444"});
    let (status, _) = patch_json(app, "/api/moderation/pending/not-a-uuid/resolve", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
