//! Tests d'integration HTTP pour /api/rbac/*.
//! Utilisent la vraie DB (sqlx direct dans le handler).

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::middleware::rbac::Role;
use sentinel_api::adapters::inbound::http::router;

fn state() -> sentinel_api::adapters::inbound::http::state::AppState {
    test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels))
}

async fn pool() -> sqlx::PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    sqlx::PgPool::connect(&url).await.unwrap()
}

// Discord ID tient dans VARCHAR(20) -> 18 digits suffisent.
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

async fn seed_user_role(pool: &sqlx::PgPool, user_id: &str, guild_id: &str, role: &str) {
    sqlx::query("INSERT INTO api_users (discord_user_id, display_name) \
                 VALUES ($1, 'test') ON CONFLICT DO NOTHING")
        .bind(user_id).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO api_user_guilds (discord_user_id, guild_id, role) \
                 VALUES ($1, $2, $3)")
        .bind(user_id).bind(guild_id).bind(role).execute(pool).await.unwrap();
}

async fn send(app: axum::Router, req: Request<Body>) -> (StatusCode, serde_json::Value) {
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

// ══════════════════════════════════════════════════════════
// grant_role (POST)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grant_role_viewer_forbidden() {
    let app = router::build_for_test(state());
    let req = test_helpers::request_with_rbac(
        "POST", &format!("/api/rbac/guilds/{}/users/{}", fresh_id(), fresh_id()),
        "caller", Some(Role::Admin), None,
        Some(serde_json::json!({"role": "viewer"})),
    );
    let (s, _) = send(app, req).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grant_role_invalid_guild_id_422() {
    let app = router::build_for_test(state());
    let req = test_helpers::request_with_rbac(
        "POST", "/api/rbac/guilds/not-a-snowflake/users/1111",
        "caller", Some(Role::Owner), None,
        Some(serde_json::json!({"role": "viewer"})),
    );
    let (s, _) = send(app, req).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grant_role_invalid_role_name_422() {
    let app = router::build_for_test(state());
    let req = test_helpers::request_with_rbac(
        "POST", &format!("/api/rbac/guilds/{}/users/{}", fresh_id(), fresh_id()),
        "caller", Some(Role::Owner), None,
        Some(serde_json::json!({"role": "superhero"})),
    );
    let (s, j) = send(app, req).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(j["error"].as_str().unwrap().contains("role invalide"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grant_role_owner_success_inserts_row() {
    let pool = pool().await;
    let app = router::build_for_test(state());
    let caller = fresh_id();
    let guild = fresh_id();
    let target = fresh_id();
    // Seed caller as owner.
    seed_user_role(&pool, &caller, &guild, "owner").await;

    let req = test_helpers::request_with_rbac(
        "POST", &format!("/api/rbac/guilds/{guild}/users/{target}"),
        &caller, Some(Role::Owner), None,
        Some(serde_json::json!({"role": "moderator", "display_name": "Alice"})),
    );
    let (s, j) = send(app, req).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["role"], "moderator");
    assert_eq!(j["granted_by"], caller);

    let (role,): (String,) = sqlx::query_as(
        "SELECT role FROM api_user_guilds WHERE discord_user_id = $1 AND guild_id = $2")
        .bind(&target).bind(&guild).fetch_one(&pool).await.unwrap();
    assert_eq!(role, "moderator");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grant_role_duplicate_returns_422_with_hint() {
    let pool = pool().await;
    let app = router::build_for_test(state());
    let guild = fresh_id();
    let target = fresh_id();
    seed_user_role(&pool, &target, &guild, "viewer").await;

    let req = test_helpers::request_with_rbac(
        "POST", &format!("/api/rbac/guilds/{guild}/users/{target}"),
        "caller", Some(Role::Owner), None,
        Some(serde_json::json!({"role": "admin"})),
    );
    let (s, j) = send(app, req).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(j["error"].as_str().unwrap().contains("PATCH"));
}

// ══════════════════════════════════════════════════════════
// update_role (PATCH)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_role_self_demotion_refused() {
    let pool = pool().await;
    let app = router::build_for_test(state());
    let guild = fresh_id();
    let caller = fresh_id();
    seed_user_role(&pool, &caller, &guild, "owner").await;

    let req = test_helpers::request_with_rbac(
        "PATCH", &format!("/api/rbac/guilds/{guild}/users/{caller}"),
        &caller, Some(Role::Owner), None,
        Some(serde_json::json!({"role": "admin"})),
    );
    let (s, j) = send(app, req).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(j["error"].as_str().unwrap().contains("retrograder"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_role_self_to_owner_allowed() {
    // Self-update vers owner est un no-op autorise.
    let pool = pool().await;
    let app = router::build_for_test(state());
    let guild = fresh_id();
    let caller = fresh_id();
    seed_user_role(&pool, &caller, &guild, "owner").await;

    let req = test_helpers::request_with_rbac(
        "PATCH", &format!("/api/rbac/guilds/{guild}/users/{caller}"),
        &caller, Some(Role::Owner), None,
        Some(serde_json::json!({"role": "owner"})),
    );
    let (s, _) = send(app, req).await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_role_unknown_user_404() {
    let app = router::build_for_test(state());
    let req = test_helpers::request_with_rbac(
        "PATCH", &format!("/api/rbac/guilds/{}/users/{}", fresh_id(), fresh_id()),
        "caller", Some(Role::Owner), None,
        Some(serde_json::json!({"role": "viewer"})),
    );
    let (s, _) = send(app, req).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_role_changes_existing_row() {
    let pool = pool().await;
    let app = router::build_for_test(state());
    let guild = fresh_id();
    let target = fresh_id();
    seed_user_role(&pool, &target, &guild, "viewer").await;

    let req = test_helpers::request_with_rbac(
        "PATCH", &format!("/api/rbac/guilds/{guild}/users/{target}"),
        "caller", Some(Role::Owner), None,
        Some(serde_json::json!({"role": "admin"})),
    );
    let (s, j) = send(app, req).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["role"], "admin");
    let (role,): (String,) = sqlx::query_as(
        "SELECT role FROM api_user_guilds WHERE discord_user_id = $1 AND guild_id = $2")
        .bind(&target).bind(&guild).fetch_one(&pool).await.unwrap();
    assert_eq!(role, "admin");
}

// ══════════════════════════════════════════════════════════
// revoke_role (DELETE)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoke_role_last_owner_refused() {
    let pool = pool().await;
    let app = router::build_for_test(state());
    let guild = fresh_id();
    let owner = fresh_id();
    seed_user_role(&pool, &owner, &guild, "owner").await;

    let req = test_helpers::request_with_rbac(
        "DELETE", &format!("/api/rbac/guilds/{guild}/users/{owner}"),
        "caller", Some(Role::Owner), None, None,
    );
    let (s, j) = send(app, req).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(j["error"].as_str().unwrap().contains("dernier owner"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoke_role_one_of_many_owners_allowed() {
    let pool = pool().await;
    let app = router::build_for_test(state());
    let guild = fresh_id();
    let owner_a = fresh_id();
    let owner_b = fresh_id();
    seed_user_role(&pool, &owner_a, &guild, "owner").await;
    seed_user_role(&pool, &owner_b, &guild, "owner").await;

    let req = test_helpers::request_with_rbac(
        "DELETE", &format!("/api/rbac/guilds/{guild}/users/{owner_a}"),
        "caller", Some(Role::Owner), None, None,
    );
    let (s, _) = send(app, req).await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoke_role_non_owner_target_never_blocks() {
    let pool = pool().await;
    let app = router::build_for_test(state());
    let guild = fresh_id();
    let target = fresh_id();
    seed_user_role(&pool, &target, &guild, "moderator").await;

    let req = test_helpers::request_with_rbac(
        "DELETE", &format!("/api/rbac/guilds/{guild}/users/{target}"),
        "caller", Some(Role::Owner), None, None,
    );
    let (s, j) = send(app, req).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoke_role_unknown_user_404() {
    let app = router::build_for_test(state());
    let req = test_helpers::request_with_rbac(
        "DELETE", &format!("/api/rbac/guilds/{}/users/{}", fresh_id(), fresh_id()),
        "caller", Some(Role::Owner), None, None,
    );
    let (s, _) = send(app, req).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

// ══════════════════════════════════════════════════════════
// list_guild_users (GET)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_guild_users_moderator_forbidden() {
    let app = router::build_for_test(state());
    let req = test_helpers::request_with_rbac(
        "GET", &format!("/api/rbac/guilds/{}/users", fresh_id()),
        "caller", Some(Role::Moderator), None, None,
    );
    let (s, _) = send(app, req).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_guild_users_ordered_by_role_then_name() {
    let pool = pool().await;
    let app = router::build_for_test(state());
    let guild = fresh_id();
    for (uid, role, name) in [
        ("u1", "viewer", "Zebra"), ("u2", "owner", "Alice"),
        ("u3", "admin", "Bob"),    ("u4", "moderator", "Chad"),
    ] {
        let id = fresh_id() + uid;
        let id = &id[..18.min(id.len())]; // tronque pour VARCHAR(20)
        sqlx::query("INSERT INTO api_users (discord_user_id, display_name) \
                     VALUES ($1, $2)")
            .bind(id).bind(name).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO api_user_guilds (discord_user_id, guild_id, role) \
                     VALUES ($1, $2, $3)")
            .bind(id).bind(&guild).bind(role).execute(&pool).await.unwrap();
    }

    let req = test_helpers::request_with_rbac(
        "GET", &format!("/api/rbac/guilds/{guild}/users"),
        "caller", Some(Role::Admin), None, None,
    );
    let (s, j) = send(app, req).await;
    assert_eq!(s, StatusCode::OK);
    let arr = j.as_array().unwrap();
    assert_eq!(arr.len(), 4);
    // Ordre : owner, admin, moderator, viewer.
    assert_eq!(arr[0]["role"], "owner");
    assert_eq!(arr[1]["role"], "admin");
    assert_eq!(arr[2]["role"], "moderator");
    assert_eq!(arr[3]["role"], "viewer");
}

// ══════════════════════════════════════════════════════════
// get_my_role (GET)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_my_role_returns_context_role() {
    let app = router::build_for_test(state());
    let guild = fresh_id();
    let req = test_helpers::request_with_rbac(
        "GET", &format!("/api/rbac/me/{guild}"),
        "caller", Some(Role::Moderator), Some(guild.clone()), None,
    );
    let (s, j) = send(app, req).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["role"], "moderator");
    assert_eq!(j["guild_id"], guild);
    assert_eq!(j["discord_user_id"], "caller");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_my_role_without_role_in_ctx_returns_404() {
    let app = router::build_for_test(state());
    let req = test_helpers::request_with_rbac(
        "GET", &format!("/api/rbac/me/{}", fresh_id()),
        "caller", None, None, None,
    );
    let (s, j) = send(app, req).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert!(j["error"].as_str().unwrap().contains("pas de role"));
}
