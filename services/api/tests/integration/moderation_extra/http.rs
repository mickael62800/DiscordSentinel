//! Tests d'integration HTTP pour les endpoints moderation evidence + review.
//!
//! Complement de `integration/moderation/http.rs` (qui teste log_action/
//! history/list_bans). Couvre add_evidence, list_evidence, add_review,
//! list_pending_reviews, resolve_review.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::adapters::inbound::http::state::AppState;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::outbound::{EvidenceEntry, EvidenceRepository, ReviewEntry, ReviewRepository};

// ══════════════════════════════════════════════════════════
// Mocks
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockEvidenceRepo {
    items: Mutex<Vec<EvidenceEntry>>,
}

#[async_trait]
impl EvidenceRepository for MockEvidenceRepo {
    async fn add(&self, action_id: Uuid, url: &str, description: Option<&str>, uploaded_by: &str, uploaded_by_name: &str) -> Result<EvidenceEntry, DomainError> {
        let e = EvidenceEntry {
            id: Uuid::new_v4(),
            action_id, url: url.into(),
            description: description.map(str::to_string),
            uploaded_by: uploaded_by.into(),
            uploaded_by_name: uploaded_by_name.into(),
            uploaded_at: Utc::now(),
        };
        self.items.lock().unwrap().push(e.clone());
        Ok(e)
    }
    async fn list(&self, action_id: Uuid) -> Result<Vec<EvidenceEntry>, DomainError> {
        Ok(self.items.lock().unwrap().iter()
            .filter(|e| e.action_id == action_id).cloned().collect())
    }
}

#[derive(Default)]
struct MockReviewRepo {
    items: Mutex<Vec<ReviewEntry>>,
    resolved: Mutex<Vec<(Uuid, String, String)>>, // (id, reviewer_id, status)
}

#[async_trait]
impl ReviewRepository for MockReviewRepo {
    async fn add(&self, action_id: Uuid, guild_id: &str, added_by: &str, added_by_name: &str, reason: Option<&str>) -> Result<ReviewEntry, DomainError> {
        let e = ReviewEntry {
            id: Uuid::new_v4(),
            action_id, guild_id: guild_id.into(),
            added_by: added_by.into(), added_by_name: added_by_name.into(),
            reason: reason.map(str::to_string),
            status: "pending".into(),
            reviewer_id: None, reviewer_name: None, reviewer_notes: None,
            added_at: Utc::now(), resolved_at: None,
            action_type: None, target_name: None, action_reason: None,
        };
        self.items.lock().unwrap().push(e.clone());
        Ok(e)
    }
    async fn list_pending(&self, guild_id: &str) -> Result<Vec<ReviewEntry>, DomainError> {
        Ok(self.items.lock().unwrap().iter()
            .filter(|r| r.guild_id == guild_id && r.status == "pending")
            .cloned().collect())
    }
    async fn resolve(&self, review_id: Uuid, reviewer_id: &str, _: &str, _: Option<&str>, status: &str) -> Result<bool, DomainError> {
        self.resolved.lock().unwrap().push((review_id, reviewer_id.into(), status.into()));
        let mut items = self.items.lock().unwrap();
        for item in items.iter_mut() {
            if item.id == review_id && item.status == "pending" {
                item.status = status.into();
                item.resolved_at = Some(Utc::now());
                return Ok(true);
            }
        }
        Ok(false)
    }
    async fn get_guild_id(&self, review_id: Uuid) -> Result<Option<String>, DomainError> {
        Ok(self.items.lock().unwrap().iter().find(|r| r.id == review_id).map(|r| r.guild_id.clone()))
    }
}

fn build_state(evidence: Arc<MockEvidenceRepo>, review: Arc<MockReviewRepo>) -> AppState {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.evidence_repo = evidence;
    state.review_repo = review;
    state
}

/// Construit un state avec un MockDiscordApi + mock moderation UC pour
/// couvrir le code apres discord_api.ban_user().await? dans execute_ban/mute/unban.
fn build_state_with_discord_mock() -> AppState {
    use sentinel_api::domain::entities::ModerationAction;
    use sentinel_api::domain::value_objects::ModerationGravity;
    use sentinel_api::ports::inbound::{
        LogModerationCommand, ManageModerationUseCase,
    };
    use sentinel_api::domain::entities::UserModerationHistory;
    use chrono::Utc;
    use async_trait::async_trait;

    struct MockModerationUC;
    #[async_trait]
    impl ManageModerationUseCase for MockModerationUC {
        async fn list_actions(&self, _: Option<&str>, _: i64) -> Result<Vec<ModerationAction>, DomainError> { Ok(vec![]) }
        async fn log_action(&self, cmd: LogModerationCommand) -> Result<ModerationAction, DomainError> {
            Ok(ModerationAction {
                id: Uuid::new_v4(),
                guild_id: cmd.guild_id, channel_id: cmd.channel_id,
                moderator_id: cmd.moderator_id, moderator_name: cmd.moderator_name,
                target_id: cmd.target_id, target_name: cmd.target_name,
                action_type: cmd.action_type, reason: cmd.reason,
                gravity: cmd.gravity.as_deref().and_then(ModerationGravity::from_str_lossy),
                duration: cmd.duration,
                created_at: Utc::now(),
            })
        }
        async fn get_history(&self, _: &str, _: &str) -> Result<UserModerationHistory, DomainError> {
            Ok(UserModerationHistory {
                target_id: String::new(), target_name: String::new(),
                total_warns: 0, total_mutes: 0, total_bans: 0, actions: vec![],
            })
        }
        async fn list_bans(&self, _: Option<&str>, _: i64, _: i64) -> Result<Vec<ModerationAction>, DomainError> { Ok(vec![]) }
        async fn delete_bans_for_user(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn delete_action(&self, _: Uuid) -> Result<bool, DomainError> { Ok(true) }
    }

    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.discord_api = Arc::new(test_helpers::MockDiscordApi::new());
    state.moderation_uc = Arc::new(MockModerationUC);
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

// ══════════════════════════════════════════════════════════
// Evidence
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_evidence_success() {
    let evidence = Arc::new(MockEvidenceRepo::default());
    let review = Arc::new(MockReviewRepo::default());
    let app = router::build_for_test(build_state(evidence.clone(), review));
    let action_id = Uuid::new_v4();
    let body = serde_json::json!({
        "action_id": action_id.to_string(),
        "url": "https://example.com/screenshot.png",
        "description": "Screenshot de l'infraction",
        "uploaded_by": "444444444444444444",
        "uploaded_by_name": "Mod Alice"
    });
    let (status, json) = post_json(app, "/api/moderation/evidence", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["url"], "https://example.com/screenshot.png");
    assert_eq!(json["uploaded_by_name"], "Mod Alice");
    assert_eq!(evidence.items.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_evidence_empty_url_422() {
    let evidence = Arc::new(MockEvidenceRepo::default());
    let review = Arc::new(MockReviewRepo::default());
    let app = router::build_for_test(build_state(evidence, review));
    let body = serde_json::json!({
        "action_id": Uuid::new_v4().to_string(),
        "url": "   ",
        "uploaded_by": "444444444444444444",
        "uploaded_by_name": "X"
    });
    let (status, _) = post_json(app, "/api/moderation/evidence", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_evidence_invalid_action_id_422() {
    let evidence = Arc::new(MockEvidenceRepo::default());
    let review = Arc::new(MockReviewRepo::default());
    let app = router::build_for_test(build_state(evidence, review));
    let body = serde_json::json!({
        "action_id": "not-a-uuid",
        "url": "https://x",
        "uploaded_by": "444444444444444444",
        "uploaded_by_name": "X"
    });
    let (status, _) = post_json(app, "/api/moderation/evidence", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_evidence_empty() {
    let evidence = Arc::new(MockEvidenceRepo::default());
    let review = Arc::new(MockReviewRepo::default());
    let app = router::build_for_test(build_state(evidence, review));
    let id = Uuid::new_v4();
    let (status, json) = get(app, &format!("/api/moderation/evidence/{id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_evidence_returns_entries() {
    let evidence = Arc::new(MockEvidenceRepo::default());
    let action_id = Uuid::new_v4();
    evidence.items.lock().unwrap().push(EvidenceEntry {
        id: Uuid::new_v4(), action_id,
        url: "https://example.com/1.png".into(), description: None,
        uploaded_by: "u1".into(), uploaded_by_name: "Alice".into(),
        uploaded_at: Utc::now(),
    });
    let review = Arc::new(MockReviewRepo::default());
    let app = router::build_for_test(build_state(evidence, review));
    let (status, json) = get(app, &format!("/api/moderation/evidence/{action_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["url"], "https://example.com/1.png");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_evidence_invalid_uuid_422() {
    let evidence = Arc::new(MockEvidenceRepo::default());
    let review = Arc::new(MockReviewRepo::default());
    let app = router::build_for_test(build_state(evidence, review));
    let (status, _) = get(app, "/api/moderation/evidence/not-a-uuid").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// Review queue
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_review_success() {
    let review = Arc::new(MockReviewRepo::default());
    let app = router::build_for_test(build_state(Arc::new(MockEvidenceRepo::default()), review.clone()));
    let body = serde_json::json!({
        "action_id": Uuid::new_v4().to_string(),
        "guild_id": "111111111111111111",
        "added_by": "444444444444444444",
        "added_by_name": "Alice",
        "reason": "Appel de l'utilisateur"
    });
    let (status, json) = post_json(app, "/api/moderation/review", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "pending");
    assert_eq!(json["reason"], "Appel de l'utilisateur");
    assert_eq!(review.items.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_review_invalid_action_id_422() {
    let review = Arc::new(MockReviewRepo::default());
    let app = router::build_for_test(build_state(Arc::new(MockEvidenceRepo::default()), review));
    let body = serde_json::json!({
        "action_id": "not-a-uuid", "guild_id": "111111111111111111",
        "added_by": "444444444444444444", "added_by_name": "X"
    });
    let (status, _) = post_json(app, "/api/moderation/review", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_pending_reviews_empty() {
    let review = Arc::new(MockReviewRepo::default());
    let app = router::build_for_test(build_state(Arc::new(MockEvidenceRepo::default()), review));
    let (status, json) = get(app, "/api/moderation/review/111111111111111111/pending").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_pending_reviews_scoped_to_guild() {
    let review = Arc::new(MockReviewRepo::default());
    let entry = ReviewEntry {
        id: Uuid::new_v4(), action_id: Uuid::new_v4(),
        guild_id: "111111111111111111".into(),
        added_by: "u1".into(), added_by_name: "A".into(),
        reason: None, status: "pending".into(),
        reviewer_id: None, reviewer_name: None, reviewer_notes: None,
        added_at: Utc::now(), resolved_at: None,
        action_type: None, target_name: None, action_reason: None,
    };
    review.items.lock().unwrap().push(entry);
    let app = router::build_for_test(build_state(Arc::new(MockEvidenceRepo::default()), review));
    let (status, json) = get(app, "/api/moderation/review/111111111111111111/pending").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_review_success() {
    let review = Arc::new(MockReviewRepo::default());
    let review_id = Uuid::new_v4();
    review.items.lock().unwrap().push(ReviewEntry {
        id: review_id, action_id: Uuid::new_v4(),
        guild_id: "111111111111111111".into(),
        added_by: "u".into(), added_by_name: "X".into(),
        reason: None, status: "pending".into(),
        reviewer_id: None, reviewer_name: None, reviewer_notes: None,
        added_at: Utc::now(), resolved_at: None,
        action_type: None, target_name: None, action_reason: None,
    });
    let app = router::build_for_test(build_state(Arc::new(MockEvidenceRepo::default()), review.clone()));
    let body = serde_json::json!({
        "status": "approved",
        "reviewer_id": "555555555555555555",
        "reviewer_name": "Admin Bob"
    });
    let (status, json) = patch_json(app, &format!("/api/moderation/review/{review_id}/resolve"), body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    let resolved = review.resolved.lock().unwrap();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].2, "approved");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_review_invalid_status_422() {
    let review = Arc::new(MockReviewRepo::default());
    let app = router::build_for_test(build_state(Arc::new(MockEvidenceRepo::default()), review));
    let id = Uuid::new_v4();
    let body = serde_json::json!({
        "status": "bogus", "reviewer_id": "555555555555555555", "reviewer_name": "X"
    });
    let (status, _) = patch_json(app, &format!("/api/moderation/review/{id}/resolve"), body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_review_not_found_returns_404() {
    let review = Arc::new(MockReviewRepo::default());
    let app = router::build_for_test(build_state(Arc::new(MockEvidenceRepo::default()), review));
    let id = Uuid::new_v4();
    let body = serde_json::json!({
        "status": "approved", "reviewer_id": "555555555555555555", "reviewer_name": "X"
    });
    let (status, _) = patch_json(app, &format!("/api/moderation/review/{id}/resolve"), body).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_review_invalid_uuid_422() {
    let review = Arc::new(MockReviewRepo::default());
    let app = router::build_for_test(build_state(Arc::new(MockEvidenceRepo::default()), review));
    let body = serde_json::json!({
        "status": "approved", "reviewer_id": "555555555555555555", "reviewer_name": "X"
    });
    let (status, _) = patch_json(app, "/api/moderation/review/not-a-uuid/resolve", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// modstats (sqlx direct -> utilise la vraie DB de test)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_modstats_empty_when_no_actions() {
    let app = router::build_for_test(build_state(
        Arc::new(MockEvidenceRepo::default()), Arc::new(MockReviewRepo::default()),
    ));
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let (status, json) = get(app, &format!("/api/moderation/modstats/{guild_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_modstats_invalid_guild_422() {
    let app = router::build_for_test(build_state(
        Arc::new(MockEvidenceRepo::default()), Arc::new(MockReviewRepo::default()),
    ));
    let (status, _) = get(app, "/api/moderation/modstats/not-a-snowflake").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// delete_action (sqlx direct + discord API)
// ══════════════════════════════════════════════════════════

async fn delete_req(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("DELETE").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_action_invalid_uuid_422() {
    let app = router::build_for_test(build_state(
        Arc::new(MockEvidenceRepo::default()), Arc::new(MockReviewRepo::default()),
    ));
    let (status, _) = delete_req(app, "/api/moderation/actions/not-a-uuid").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_action_not_found_returns_404() {
    let app = router::build_for_test(build_state(
        Arc::new(MockEvidenceRepo::default()), Arc::new(MockReviewRepo::default()),
    ));
    let id = Uuid::new_v4();
    let (status, _) = delete_req(app, &format!("/api/moderation/actions/{id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ══════════════════════════════════════════════════════════
// execute_ban / execute_mute / execute_unban
// (Discord API non configure -> 500, mais validation + flux ok)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_ban_without_token_returns_500() {
    let app = router::build_for_test(build_state(
        Arc::new(MockEvidenceRepo::default()), Arc::new(MockReviewRepo::default()),
    ));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444",
        "reason": "Spam repete"
    });
    let (status, _) = post_json(app, "/api/moderation/execute-ban", body).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_ban_success_with_mock_discord() {
    let app = router::build_for_test(build_state_with_discord_mock());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444",
        "reason": "Spam"
    });
    let (status, json) = post_json(app, "/api/moderation/execute-ban", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_mute_success_with_mock_discord() {
    let app = router::build_for_test(build_state_with_discord_mock());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444",
        "reason": "Flood",
        "duration": 1800,
        "target_name": "alice"
    });
    let (status, json) = post_json(app, "/api/moderation/execute-mute", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_mute_success_default_duration() {
    // Path default 3600s quand duration absent.
    let app = router::build_for_test(build_state_with_discord_mock());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444",
        "reason": "r"
    });
    let (status, _) = post_json(app, "/api/moderation/execute-mute", body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_unban_success_with_mock_discord() {
    let app = router::build_for_test(build_state_with_discord_mock());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444"
    });
    let (status, json) = post_json(app, "/api/moderation/execute-unban", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

// ══════════════════════════════════════════════════════════
// delete_action : insert une vraie ligne puis supprime-la
// (couvre les branches ban*/mute*/default du reversal Discord)
// ══════════════════════════════════════════════════════════

async fn insert_action(pool: &sqlx::PgPool, guild_id: &str, action_type: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO moderation_actions \
         (id, guild_id, channel_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, created_at) \
         VALUES ($1, $2, '555555555555555555', 'desktop', 'Desktop', '444444444444444444', 'Alice', $3, 'test', NOW())",
    )
    .bind(id)
    .bind(guild_id)
    .bind(action_type)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn pool() -> sqlx::PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    sqlx::PgPool::connect(&url).await.unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_action_ban_triggers_discord_unban_and_succeeds() {
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let p = pool().await;
    let id = insert_action(&p, &guild_id, "ban_permanent").await;
    let app = router::build_for_test(build_state_with_discord_mock());
    let (status, _) = delete_req(app, &format!("/api/moderation/actions/{id}")).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_action_mute_triggers_discord_remove_timeout() {
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let p = pool().await;
    let id = insert_action(&p, &guild_id, "mute_temp").await;
    let app = router::build_for_test(build_state_with_discord_mock());
    let (status, _) = delete_req(app, &format!("/api/moderation/actions/{id}")).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_action_warn_no_discord_call() {
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let p = pool().await;
    let id = insert_action(&p, &guild_id, "warn").await;
    let app = router::build_for_test(build_state_with_discord_mock());
    let (status, _) = delete_req(app, &format!("/api/moderation/actions/{id}")).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_ban_invalid_guild_422() {
    let app = router::build_for_test(build_state(
        Arc::new(MockEvidenceRepo::default()), Arc::new(MockReviewRepo::default()),
    ));
    let body = serde_json::json!({"guild_id": "bad", "user_id": "444444444444444444", "reason": "r"});
    let (status, _) = post_json(app, "/api/moderation/execute-ban", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_mute_without_token_returns_500() {
    let app = router::build_for_test(build_state(
        Arc::new(MockEvidenceRepo::default()), Arc::new(MockReviewRepo::default()),
    ));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444",
        "reason": "Flood",
        "duration": 1800
    });
    let (status, _) = post_json(app, "/api/moderation/execute-mute", body).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_mute_default_duration_1h() {
    // Sans champ duration -> handler applique 3600s par defaut.
    let app = router::build_for_test(build_state(
        Arc::new(MockEvidenceRepo::default()), Arc::new(MockReviewRepo::default()),
    ));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444",
        "reason": "r"
    });
    let (status, _) = post_json(app, "/api/moderation/execute-mute", body).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_unban_without_token_returns_500() {
    let app = router::build_for_test(build_state(
        Arc::new(MockEvidenceRepo::default()), Arc::new(MockReviewRepo::default()),
    ));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444"
    });
    let (status, _) = post_json(app, "/api/moderation/execute-unban", body).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_unban_invalid_path_422() {
    let app = router::build_for_test(build_state(
        Arc::new(MockEvidenceRepo::default()), Arc::new(MockReviewRepo::default()),
    ));
    let body = serde_json::json!({"guild_id": "bad", "user_id": "x"});
    let (status, _) = post_json(app, "/api/moderation/execute-unban", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
