//! Tests d'integration HTTP pour les endpoints discord-roles.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::adapters::inbound::http::state::AppState;
use sentinel_core::domain::entities::system::discord_role::DiscordRole;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::community::discord_role_repository::DiscordRoleRepository;

// ══════════════════════════════════════════════════════════
// Mock DiscordRoleRepository
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockDiscordRoleRepo {
    roles: Mutex<Vec<DiscordRole>>,
    synced: Mutex<Vec<(String, Vec<DiscordRole>)>>,
}

impl MockDiscordRoleRepo {
    fn new() -> Self {
        Self::default()
    }
    fn with(self, r: DiscordRole) -> Self {
        self.roles.lock().unwrap().push(r);
        self
    }
}

fn sample_role(guild_id: &str, id: &str, permissions: i64) -> DiscordRole {
    DiscordRole {
        id: id.into(),
        guild_id: guild_id.into(),
        name: "Admins".into(),
        color: 0x3498db,
        position: 1,
        permissions,
        mentionable: true,
        managed: false,
        icon: None,
        member_count: 3,
        synced_at: Utc::now(),
    }
}

#[async_trait]
impl DiscordRoleRepository for MockDiscordRoleRepo {
    async fn sync_roles(&self, guild_id: &str, roles: Vec<DiscordRole>) -> Result<(), DomainError> {
        self.synced.lock().unwrap().push((guild_id.into(), roles));
        Ok(())
    }
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<DiscordRole>, DomainError> {
        Ok(self
            .roles
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.guild_id == guild_id.into())
            .cloned()
            .collect())
    }
    async fn find_by_id(
        &self,
        guild_id: &str,
        role_id: &str,
    ) -> Result<Option<DiscordRole>, DomainError> {
        Ok(self
            .roles
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.guild_id == guild_id.into() && r.id == role_id)
            .cloned())
    }
}

fn build_state(repo: Arc<MockDiscordRoleRepo>) -> AppState {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.community.discord_role_repo = repo;
    state.discord_api = Arc::new(test_helpers::MockDiscordApi::new());
    state
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
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
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

async fn patch_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

async fn delete(app: axum::Router, uri: &str) -> StatusCode {
    let req = Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    resp.status()
}
// ══════════════════════════════════════════════════════════
// list_roles
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_roles_empty() {
    let app = router::build_for_test(build_state(Arc::new(MockDiscordRoleRepo::new())));
    let (status, json) = get(app, "/api/discord-roles/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_roles_serializes_permissions_as_string() {
    // Bigint permissions > 2^53 → doit etre string dans le JSON.
    let repo =
        MockDiscordRoleRepo::new().with(sample_role("111111111111111111", "r1", 9007199254740993));
    let app = router::build_for_test(build_state(Arc::new(repo)));
    let (status, json) = get(app, "/api/discord-roles/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["permissions"], "9007199254740993");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_roles_scoped_to_guild() {
    let repo = MockDiscordRoleRepo::new()
        .with(sample_role("111111111111111111", "r1", 8))
        .with(sample_role("222222222222222222", "r2", 16));
    let app = router::build_for_test(build_state(Arc::new(repo)));
    let (status, json) = get(app, "/api/discord-roles/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
}

// ══════════════════════════════════════════════════════════
// create_role / edit_role (via MockDiscordApi)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_role_success_via_mock_discord() {
    let app = router::build_for_test(build_state(Arc::new(MockDiscordRoleRepo::new())));
    let body = serde_json::json!({
        "name": "New Role", "color": 0x3498db, "permissions": "8"
    });
    let (status, json) = post_json(app, "/api/discord-roles/111111111111111111/create", body).await;
    assert_eq!(status, StatusCode::OK);
    // MockDiscordApi.create_role retourne {"id": "r1", "name": "role"}
    assert_eq!(json["id"], "r1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_role_success_via_mock_discord() {
    let app = router::build_for_test(build_state(Arc::new(MockDiscordRoleRepo::new())));
    let body = serde_json::json!({
        "name": "Renamed", "color": 0xff0000, "mentionable": true
    });
    let (status, json) = patch_json(app, "/api/discord-roles/111111111111111111/555", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["id"], "r1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_role_accepts_partial_body() {
    // Tous les champs sont optionnels — body vide accepte.
    let app = router::build_for_test(build_state(Arc::new(MockDiscordRoleRepo::new())));
    let body = serde_json::json!({});
    let (status, _) = patch_json(app, "/api/discord-roles/111111111111111111/555", body).await;
    assert_eq!(status, StatusCode::OK);
}

// ══════════════════════════════════════════════════════════
// delete_role (RBAC admin+)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_role_without_rbac_passes_through() {
    // Sans rbac header -> pass-through (bot/internal).
    let app = router::build_for_test(build_state(Arc::new(MockDiscordRoleRepo::new())));
    let status = delete(app, "/api/discord-roles/111111111111111111/555").await;
    assert_eq!(status, StatusCode::OK);
}
// ══════════════════════════════════════════════════════════
// sync_roles
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_roles_parses_permissions_and_persists() {
    let repo = Arc::new(MockDiscordRoleRepo::new());
    let app = router::build_for_test(build_state(repo.clone()));
    let body = serde_json::json!({
        "roles": [
            {
                "id": "r1", "name": "Admins", "color": 0x3498db, "position": 10,
                "permissions": "8", "mentionable": true, "managed": false,
                "icon": null, "member_count": 3
            },
            {
                "id": "r2", "name": "Mods", "color": 0x00ff00, "position": 5,
                "permissions": "9007199254740993", "mentionable": false, "managed": false,
                "icon": null, "member_count": 5
            }
        ]
    });
    let (status, json) = post_json(app, "/api/discord-roles/111111111111111111/sync", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["synced"], 2);

    let synced = repo.synced.lock().unwrap();
    assert_eq!(synced[0].0, "111111111111111111");
    assert_eq!(synced[0].1.len(), 2);
    assert_eq!(synced[0].1[0].permissions, 8);
    assert_eq!(synced[0].1[1].permissions, 9007199254740993);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_roles_invalid_permissions_falls_back_to_zero() {
    // Regle metier : parse_discord_permissions_bitfield retourne 0 sur input invalide.
    let repo = Arc::new(MockDiscordRoleRepo::new());
    let app = router::build_for_test(build_state(repo.clone()));
    let body = serde_json::json!({
        "roles": [
            {
                "id": "r1", "name": "X", "color": 0, "position": 0,
                "permissions": "not-a-number", "mentionable": false, "managed": false,
                "icon": null, "member_count": 0
            }
        ]
    });
    let (status, _) = post_json(app, "/api/discord-roles/111111111111111111/sync", body).await;
    assert_eq!(status, StatusCode::OK);
    let synced = repo.synced.lock().unwrap();
    assert_eq!(synced[0].1[0].permissions, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_roles_empty_list_ok() {
    let repo = Arc::new(MockDiscordRoleRepo::new());
    let app = router::build_for_test(build_state(repo.clone()));
    let body = serde_json::json!({"roles": []});
    let (status, json) = post_json(app, "/api/discord-roles/111111111111111111/sync", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["synced"], 0);
}
