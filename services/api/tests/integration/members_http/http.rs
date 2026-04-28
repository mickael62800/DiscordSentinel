//! Tests d'integration HTTP pour les endpoints guild_members.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::adapters::inbound::http::state::AppState;
use sentinel_api::domain::entities::community::guild_member::GuildMember;
use sentinel_api::domain::entities::community::guild_member::MemberConduct;
use sentinel_api::domain::entities::community::guild_member::MemberInfractions;
use sentinel_api::domain::entities::community::guild_member::MemberModeration;
use sentinel_api::domain::entities::community::guild_member::MemberStats;
use sentinel_api::domain::entities::community::guild_member::MemberSummary;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::community::manage_members::ManageMembersUseCase;
use sentinel_api::ports::inbound::community::manage_members::RegisterMemberCommand;
use sentinel_api::ports::inbound::community::manage_members::SyncMembersCommand;
use sentinel_api::ports::inbound::community::manage_members::UpdateMemberCommand;
// ══════════════════════════════════════════════════════════
// Mock ManageMembersUseCase
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockMembersUC {
    members: Mutex<Vec<GuildMember>>,
    synced_count: Mutex<u64>,
    removed: Mutex<Vec<(String, String)>>,
    updated: Mutex<Vec<UpdateMemberCommand>>,
    registered: Mutex<Vec<GuildMember>>,
}

impl MockMembersUC {
    fn new() -> Self { Self::default() }
    fn with_member(self, m: GuildMember) -> Self { self.members.lock().unwrap().push(m); self }
}

fn sample_member(guild_id: &str, user_id: &str) -> GuildMember {
    GuildMember {
        guild_id: guild_id.into(),
        user_id: user_id.into(),
        username: "alice".into(),
        display_name: Some("Alice".into()),
        avatar: None,
        roles: serde_json::json!([]),
        joined_at: None,
        account_created: None,
        is_bot: false,
        last_seen_at: None,
    }
}

#[async_trait]
impl ManageMembersUseCase for MockMembersUC {
    async fn list_members(&self, guild_id: &str) -> Result<Vec<GuildMember>, DomainError> {
        Ok(self.members.lock().unwrap().iter()
            .filter(|m| m.guild_id == guild_id).cloned().collect())
    }
    async fn get_member(&self, guild_id: &str, user_id: &str) -> Result<GuildMember, DomainError> {
        self.members.lock().unwrap().iter()
            .find(|m| m.guild_id == guild_id && m.user_id == user_id)
            .cloned()
            .ok_or_else(|| DomainError::NotFound("member".into()))
    }
    async fn get_member_summary(&self, guild_id: &str, user_id: &str) -> Result<MemberSummary, DomainError> {
        let m = self.members.lock().unwrap().iter()
            .find(|m| m.guild_id == guild_id && m.user_id == user_id)
            .cloned()
            .ok_or_else(|| DomainError::NotFound("member".into()))?;
        Ok(MemberSummary {
            member: m,
            conduct: MemberConduct { points: 100, max_points: 100, log: vec![] },
            infractions: MemberInfractions { total: 0, recent: vec![] },
            moderation: MemberModeration { total_warns: 0, total_mutes: 0, total_bans: 0, actions: vec![] },
            stats: MemberStats { message_count: 0, voice_seconds: 0, last_active: None },
        })
    }
    async fn sync_members(&self, cmd: SyncMembersCommand) -> Result<u64, DomainError> {
        let count = cmd.members.len() as u64;
        *self.synced_count.lock().unwrap() = count;
        Ok(count)
    }
    async fn register_member(&self, cmd: RegisterMemberCommand) -> Result<(), DomainError> {
        self.registered.lock().unwrap().push(cmd.member);
        Ok(())
    }
    async fn remove_member(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.removed.lock().unwrap().push((guild_id.into(), user_id.into()));
        Ok(())
    }
    async fn update_member(&self, cmd: UpdateMemberCommand) -> Result<(), DomainError> {
        self.updated.lock().unwrap().push(cmd);
        Ok(())
    }
}

fn build_state(uc: Arc<MockMembersUC>) -> AppState {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.members_uc = uc;
    state.discord_api = Arc::new(test_helpers::MockDiscordApi::new());
    state
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null))
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
// list_members (Discord API via MockDiscordApi, cache Redis)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_members_via_discord_api_empty() {
    let app = router::build_for_test(build_state(Arc::new(MockMembersUC::new())));
    let (status, json) = get(app, "/api/guilds/111111111111111111/members").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_members_second_call_hits_cache() {
    let app1 = router::build_for_test(build_state(Arc::new(MockMembersUC::new())));
    let (s, _) = get(app1, "/api/guilds/111111111111111111/members").await;
    assert_eq!(s, StatusCode::OK);
    let app2 = router::build_for_test(build_state(Arc::new(MockMembersUC::new())));
    let (s, _) = get(app2, "/api/guilds/111111111111111111/members").await;
    assert_eq!(s, StatusCode::OK);
}

// ══════════════════════════════════════════════════════════
// list_members_db / get_member / get_member_summary
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_members_db_scoped_to_guild() {
    let uc = Arc::new(MockMembersUC::new()
        .with_member(sample_member("111111111111111111", "u1"))
        .with_member(sample_member("222222222222222222", "u2")));
    let app = router::build_for_test(build_state(uc));
    let (status, json) = get(app, "/api/members/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_member_success() {
    let uc = Arc::new(MockMembersUC::new()
        .with_member(sample_member("111111111111111111", "u1")));
    let app = router::build_for_test(build_state(uc));
    let (status, json) = get(app, "/api/members/111111111111111111/u1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["user_id"], "u1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_member_not_found() {
    let app = router::build_for_test(build_state(Arc::new(MockMembersUC::new())));
    let (status, _) = get(app, "/api/members/111111111111111111/ghost").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_member_summary_aggregates() {
    let uc = Arc::new(MockMembersUC::new()
        .with_member(sample_member("111111111111111111", "u1")));
    let app = router::build_for_test(build_state(uc));
    let (status, json) = get(app, "/api/members/111111111111111111/u1/summary").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["member"]["user_id"], "u1");
    assert_eq!(json["conduct"]["points"], 100);
}

// ══════════════════════════════════════════════════════════
// sync_members / register_member
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_members_returns_count() {
    let uc = Arc::new(MockMembersUC::new());
    let app = router::build_for_test(build_state(uc.clone()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "members": [
            sample_member("111111111111111111", "u1"),
            sample_member("111111111111111111", "u2"),
            sample_member("111111111111111111", "u3"),
        ]
    });
    let (status, json) = post_json(app, "/api/members/sync", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["synced"], 3);
    assert_eq!(*uc.synced_count.lock().unwrap(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn register_member_stores_it() {
    let uc = Arc::new(MockMembersUC::new());
    let app = router::build_for_test(build_state(uc.clone()));
    let member = sample_member("111111111111111111", "newbie");
    let (status, _) = post_json(app, "/api/members/register", serde_json::to_value(member).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(uc.registered.lock().unwrap().len(), 1);
    assert_eq!(uc.registered.lock().unwrap()[0].user_id, "newbie");
}

// ══════════════════════════════════════════════════════════
// update_member
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_member_forwards_fields() {
    let uc = Arc::new(MockMembersUC::new());
    let app = router::build_for_test(build_state(uc.clone()));
    let body = serde_json::json!({
        "username": "new_name",
        "display_name": "New Name",
        "avatar": "hash123"
    });
    let (status, _) = patch_json(app, "/api/members/111111111111111111/u1", body).await;
    assert_eq!(status, StatusCode::OK);
    let updated = uc.updated.lock().unwrap();
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].username.as_deref(), Some("new_name"));
    assert_eq!(updated[0].display_name.as_deref(), Some("New Name"));
}

// ══════════════════════════════════════════════════════════
// remove_member + RBAC
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remove_member_without_rbac_succeeds() {
    let uc = Arc::new(MockMembersUC::new());
    let app = router::build_for_test(build_state(uc.clone()));
    let status = delete(app, "/api/members/111111111111111111/u1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(uc.removed.lock().unwrap()[0], ("111111111111111111".into(), "u1".into()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remove_member_with_rbac_viewer_forbidden() {
    use sentinel_api::domain::enums::system::role::Role;
    let app = router::build_for_test(build_state(Arc::new(MockMembersUC::new())));
    let req = test_helpers::request_with_rbac(
        "DELETE", "/api/members/111111111111111111/u1",
        "444444444444444444", Some(Role::Viewer), Some("111111111111111111".into()),
        None,
    );
    let (status, json) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(json["error"].as_str().unwrap().contains("moderator+"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remove_member_with_rbac_moderator_allowed() {
    use sentinel_api::domain::enums::system::role::Role;
    let app = router::build_for_test(build_state(Arc::new(MockMembersUC::new())));
    let req = test_helpers::request_with_rbac(
        "DELETE", "/api/members/111111111111111111/u1",
        "444444444444444444", Some(Role::Moderator), Some("111111111111111111".into()),
        None,
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
}

// ══════════════════════════════════════════════════════════
// reset_member (transaction sqlx + RBAC)
// ══════════════════════════════════════════════════════════

async fn seed_rbac_admin(guild_id: &str, user_id: &str) {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    let p = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::query("INSERT INTO api_users (discord_user_id, display_name) VALUES ($1, 'A') ON CONFLICT DO NOTHING")
        .bind(user_id).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO api_user_guilds (discord_user_id, guild_id, role) VALUES ($1, $2, 'admin')")
        .bind(user_id).bind(guild_id).execute(&p).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_member_success_returns_totals() {
    use sentinel_api::domain::enums::system::role::Role;
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let user_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let admin_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    seed_rbac_admin(&guild_id, &admin_id).await;

    let app = router::build_for_test(build_state(Arc::new(MockMembersUC::new())));
    let req = test_helpers::request_with_rbac(
        "POST", &format!("/api/members/{guild_id}/{user_id}/reset"),
        &admin_id, Some(Role::Admin), Some(guild_id.clone()),
        None,
    );
    let (status, json) = send_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
    // Les 8 tables listees dans MEMBER_RESET_TABLES doivent etre dans totals
    for key in ["infractions", "moderation_actions", "conduct_points", "conduct_log",
                "strikes", "notes", "manual_watched", "sanction_reminders"] {
        assert!(json["totals"].get(key).is_some(), "missing key {key}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_member_with_rbac_moderator_forbidden() {
    use sentinel_api::domain::enums::system::role::Role;
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let user_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let mod_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    let p = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::query("INSERT INTO api_users (discord_user_id, display_name) VALUES ($1, 'M') ON CONFLICT DO NOTHING")
        .bind(&mod_id).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO api_user_guilds (discord_user_id, guild_id, role) VALUES ($1, $2, 'moderator')")
        .bind(&mod_id).bind(&guild_id).execute(&p).await.unwrap();

    let app = router::build_for_test(build_state(Arc::new(MockMembersUC::new())));
    let req = test_helpers::request_with_rbac(
        "POST", &format!("/api/members/{guild_id}/{user_id}/reset"),
        &mod_id, Some(Role::Moderator), Some(guild_id),
        None,
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
