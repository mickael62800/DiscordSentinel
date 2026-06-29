//! Tests d'integration HTTP pour les endpoints infractions.

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
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::ports::inbound::moderation::manage_infractions::InfractionFilters;
use sentinel_api::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use sentinel_core::domain::entities::moderation::detection_flags::DetectionFlags;
use sentinel_core::domain::entities::moderation::infraction::Infraction;
use sentinel_core::domain::enums::moderation::action::Action;
use sentinel_core::domain::errors::DomainError;
use test_helpers::build_test_state_infractions;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockInfractionsUC {
    items: Mutex<Vec<Infraction>>,
}

impl MockInfractionsUC {
    fn new() -> Self {
        Self::default()
    }
    fn with(self, inf: Infraction) -> Self {
        self.items.lock().unwrap().push(inf);
        self
    }
}

#[async_trait]
impl ManageInfractionsUseCase for MockInfractionsUC {
    async fn list_infractions(
        &self,
        guild_id: &str,
        filters: InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError> {
        let all = self.items.lock().unwrap();
        let matching: Vec<Infraction> = all
            .iter()
            .filter(|i| i.guild_id == guild_id)
            .filter(|i| filters.user_id.as_deref().is_none_or(|u| i.user_id == u))
            .filter(|i| {
                filters
                    .action
                    .as_deref()
                    .is_none_or(|a| i.action.as_str() == a)
            })
            .skip(filters.offset as usize)
            .take(filters.limit as usize)
            .cloned()
            .collect();
        Ok(matching)
    }
    async fn list_all_infractions(&self, _: i64, _: i64) -> Result<Vec<Infraction>, DomainError> {
        Ok(self.items.lock().unwrap().clone())
    }
    async fn count_today(&self) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn find_by_id(&self, id: &str) -> Result<Option<Infraction>, DomainError> {
        let uuid =
            Uuid::parse_str(id).map_err(|_| DomainError::ValidationError("bad uuid".into()))?;
        Ok(self
            .items
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id == uuid)
            .cloned())
    }
    async fn delete_infraction(&self, id: &str) -> Result<bool, DomainError> {
        let uuid = match Uuid::parse_str(id) {
            Ok(u) => u,
            Err(_) => return Ok(false),
        };
        let mut items = self.items.lock().unwrap();
        let before = items.len();
        items.retain(|i| i.id != uuid);
        Ok(before != items.len())
    }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> {
        Ok(0)
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_app(uc: MockInfractionsUC) -> axum::Router {
    router::build_for_test(build_test_state_infractions(Arc::new(uc)))
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

async fn delete(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

fn sample_infraction(guild_id: &str, user_id: &str, action: Action) -> Infraction {
    Infraction {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        channel_id: "555555555555555555".into(),
        user_id: user_id.into(),
        username: "alice".into(),
        message_id: "666666666666666666".into(),
        content: "bad".into(),
        flags: DetectionFlags {
            spam: false,
            insult: false,
            link: false,
            phishing: false,
        },
        score: 0.8,
        action,
        reason: "reason".into(),
        duration: None,
        created_at: Utc::now(),
    }
}

// ══════════════════════════════════════════════════════════
// GET /infractions/{guild_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_infractions_empty() {
    let app = build_app(MockInfractionsUC::new());
    let (status, json) = get(app, "/infractions/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_infractions_scoped_to_guild() {
    let uc = MockInfractionsUC::new()
        .with(sample_infraction("111111111111111111", "u1", Action::Warn))
        .with(sample_infraction("222222222222222222", "u1", Action::Ban));
    let app = build_app(uc);
    let (status, json) = get(app, "/infractions/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["action"], "warn");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_infractions_filter_by_user_id() {
    let uc = MockInfractionsUC::new()
        .with(sample_infraction("111111111111111111", "u1", Action::Warn))
        .with(sample_infraction("111111111111111111", "u2", Action::Mute));
    let app = build_app(uc);
    let (status, json) = get(app, "/infractions/111111111111111111?user_id=u1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["user_id"], "u1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_infractions_filter_by_action() {
    let uc = MockInfractionsUC::new()
        .with(sample_infraction("111111111111111111", "u1", Action::Warn))
        .with(sample_infraction("111111111111111111", "u2", Action::Ban));
    let app = build_app(uc);
    let (status, json) = get(app, "/infractions/111111111111111111?action=ban").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["action"], "ban");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_infractions_validates_guild_id() {
    let app = build_app(MockInfractionsUC::new());
    let (status, _) = get(app, "/infractions/not-a-snowflake").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_infractions_respects_limit() {
    let mut uc = MockInfractionsUC::new();
    for _ in 0..5 {
        uc = uc.with(sample_infraction("111111111111111111", "u1", Action::Warn));
    }
    let app = build_app(uc);
    let (status, json) = get(app, "/infractions/111111111111111111?limit=2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 2);
}

// ══════════════════════════════════════════════════════════
// DELETE /infractions/delete/{id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_infraction_not_found_returns_404() {
    let app = build_app(MockInfractionsUC::new());
    let id = Uuid::new_v4();
    let (status, _) = delete(app, &format!("/infractions/delete/{id}")).await;
    // find_by_id returns None → handler continue et delete_infraction renvoie false → 404
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_infraction_success_returns_ok_true() {
    let inf = sample_infraction("111111111111111111", "u1", Action::Warn);
    let id = inf.id;
    let app = build_app(MockInfractionsUC::new().with(inf));
    let (status, json) = delete(app, &format!("/infractions/delete/{id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}
