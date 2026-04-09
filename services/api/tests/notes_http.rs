//! Tests d'integration HTTP pour les endpoints notes.

mod test_helpers;

use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::domain::entities::*;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::*;

// ══════════════════════════════════════════════════════════
// Mock Notes Use Case
// ══════════════════════════════════════════════════════════

struct MockNotesUC {
    notes: Vec<UserNote>,
}

impl MockNotesUC {
    fn new() -> Self {
        Self { notes: vec![] }
    }

    fn with_note(mut self, n: UserNote) -> Self {
        self.notes.push(n);
        self
    }
}

fn make_note(guild_id: &str, user_id: &str, content: &str, category: &str) -> UserNote {
    UserNote {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        user_id: user_id.into(),
        author_id: "mod1".into(),
        author_name: "Bob".into(),
        content: content.into(),
        category: category.into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[async_trait]
impl ManageNotesUseCase for MockNotesUC {
    async fn add_note(&self, cmd: AddNoteCommand) -> Result<UserNote, DomainError> {
        let valid = ["general", "warning", "positive", "context"];
        if !valid.contains(&cmd.category.as_str()) {
            return Err(DomainError::ValidationError(format!("Categorie invalide '{}'", cmd.category)));
        }
        Ok(UserNote {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            user_id: cmd.user_id,
            author_id: cmd.author_id,
            author_name: cmd.author_name,
            content: cmd.content,
            category: cmd.category,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    async fn get_notes(&self, guild_id: &str, user_id: &str) -> Result<Vec<UserNote>, DomainError> {
        Ok(self.notes.iter()
            .filter(|n| n.guild_id == guild_id && n.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn delete_note(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_app(uc: MockNotesUC) -> axum::Router {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.notes_uc = Arc::new(uc);
    router::build_for_test(state)
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null))
}

async fn post_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("POST").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

async fn delete_req(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("DELETE").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null))
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — POST /api/notes
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_note_success() {
    let app = build_app(MockNotesUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444",
        "author_id": "mod1",
        "author_name": "Bob",
        "content": "Comportement suspect a surveiller",
        "category": "warning"
    });
    let (status, json) = post_json(app, "/api/notes", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["guild_id"], "111111111111111111");
    assert_eq!(json["user_id"], "444444444444444444");
    assert_eq!(json["content"], "Comportement suspect a surveiller");
    assert_eq!(json["category"], "warning");
    assert!(json["id"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_note_default_category() {
    let app = build_app(MockNotesUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444",
        "author_id": "mod1",
        "author_name": "Bob",
        "content": "Note simple"
    });
    let (status, json) = post_json(app, "/api/notes", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["category"], "general");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_note_invalid_category_returns_422() {
    let app = build_app(MockNotesUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "user_id": "444444444444444444",
        "author_id": "mod1",
        "author_name": "Bob",
        "content": "Test",
        "category": "invalid_cat"
    });
    let (status, _) = post_json(app, "/api/notes", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_note_missing_fields_returns_422() {
    let app = build_app(MockNotesUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111"
    });
    let (status, _) = post_json(app, "/api/notes", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — GET /api/notes/{guild_id}/{user_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_notes_empty() {
    let app = build_app(MockNotesUC::new());
    let (status, json) = get(app, "/api/notes/111111111111111111/444444444444444444").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_notes_with_data() {
    let uc = MockNotesUC::new()
        .with_note(make_note("111111111111111111", "444444444444444444", "Note 1", "general"))
        .with_note(make_note("111111111111111111", "444444444444444444", "Note 2", "warning"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/notes/111111111111111111/444444444444444444").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 2);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — DELETE /api/notes/{id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_note_success() {
    let app = build_app(MockNotesUC::new());
    let (status, json) = delete_req(app, &format!("/api/notes/{}", Uuid::new_v4())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}
