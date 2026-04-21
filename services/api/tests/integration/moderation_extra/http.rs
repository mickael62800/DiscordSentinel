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
