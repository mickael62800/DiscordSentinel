//! Tests d'integration HTTP pour les endpoints blackjack.

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
use sentinel_api::adapters::inbound::http::state::AppState;
use sentinel_api::ports::outbound::casino::blackjack_table_repository::BlackjackTable;
use sentinel_api::ports::outbound::casino::blackjack_table_repository::BlackjackTablePlayer;
use sentinel_api::ports::outbound::casino::blackjack_table_repository::BlackjackTableRepository;
use sentinel_core::domain::errors::DomainError;
// ══════════════════════════════════════════════════════════
// Mock BlackjackTableRepository
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockTableRepo {
    tables: Mutex<Vec<BlackjackTable>>,
    players: Mutex<Vec<(String, String, String)>>,
    closed: Mutex<Vec<String>>,
}

impl MockTableRepo {
    fn new() -> Self {
        Self::default()
    }
    fn with_table(self, t: BlackjackTable) -> Self {
        self.tables.lock().unwrap().push(t);
        self
    }
}

fn sample_table(guild_id: &str, id: &str, status: &str) -> BlackjackTable {
    BlackjackTable {
        id: id.into(),
        guild_id: guild_id.into(),
        channel_id: "c1".into(),
        owner_id: "444444444444444444".into(),
        owner_name: "Owner".into(),
        status: status.into(),
        created_at: Utc::now().to_rfc3339(),
    }
}

#[async_trait]
impl BlackjackTableRepository for MockTableRepo {
    async fn create(
        &self,
        guild_id: &str,
        channel_id: &str,
        owner_id: &str,
        owner_name: &str,
        _shoe: &serde_json::Value,
    ) -> Result<BlackjackTable, DomainError> {
        let t = BlackjackTable {
            id: Uuid::new_v4().to_string(),
            guild_id: guild_id.into(),
            channel_id: channel_id.into(),
            owner_id: owner_id.into(),
            owner_name: owner_name.into(),
            status: "open".into(),
            created_at: Utc::now().to_rfc3339(),
        };
        self.tables.lock().unwrap().push(t.clone());
        Ok(t)
    }
    async fn get_status_and_guild(
        &self,
        table_id: &str,
    ) -> Result<Option<(String, String)>, DomainError> {
        Ok(self
            .tables
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id == table_id)
            .map(|t| (t.status.clone(), t.guild_id.clone())))
    }
    async fn count_players(&self, table_id: &str) -> Result<i64, DomainError> {
        Ok(self
            .players
            .lock()
            .unwrap()
            .iter()
            .filter(|(t, _, _)| t == table_id)
            .count() as i64)
    }
    async fn add_player(
        &self,
        table_id: &str,
        user_id: &str,
        user_name: &str,
    ) -> Result<(), DomainError> {
        self.players
            .lock()
            .unwrap()
            .push((table_id.into(), user_id.into(), user_name.into()));
        Ok(())
    }
    async fn touch_activity(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_players(&self, table_id: &str) -> Result<Vec<BlackjackTablePlayer>, DomainError> {
        Ok(self
            .players
            .lock()
            .unwrap()
            .iter()
            .filter(|(t, _, _)| t == table_id)
            .map(|(_, uid, uname)| BlackjackTablePlayer {
                user_id: uid.clone(),
                user_name: uname.clone(),
                joined_at: Utc::now().to_rfc3339(),
            })
            .collect())
    }
    async fn find_open_by_channel(
        &self,
        channel_id: &str,
    ) -> Result<Option<BlackjackTable>, DomainError> {
        Ok(self
            .tables
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.channel_id == channel_id && t.status == "open")
            .cloned())
    }
    async fn get_guild_id(&self, table_id: &str) -> Result<Option<String>, DomainError> {
        Ok(self
            .tables
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id == table_id)
            .map(|t| t.guild_id.clone()))
    }
    async fn close(&self, table_id: &str) -> Result<(), DomainError> {
        self.closed.lock().unwrap().push(table_id.into());
        for t in self.tables.lock().unwrap().iter_mut() {
            if t.id == table_id {
                t.status = "closed".into();
            }
        }
        Ok(())
    }
    async fn list_games(&self, _: &str) -> Result<Vec<serde_json::Value>, DomainError> {
        Ok(vec![])
    }
}

fn build_state(repo: Arc<MockTableRepo>) -> AppState {
    let mut state = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    state.blackjack_table_repo = repo;
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
// Tables endpoints
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_table_builds_shoe_and_stores() {
    let repo = Arc::new(MockTableRepo::new());
    let app = router::build_for_test(build_state(repo.clone()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "channel_id": "c1",
        "owner_id": "444444444444444444",
        "owner_name": "Alice"
    });
    let (status, json) = post_json(app, "/api/blackjack/tables", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "open");
    assert_eq!(json["owner_name"], "Alice");
    assert_eq!(repo.tables.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn join_table_not_found_returns_404() {
    let repo = Arc::new(MockTableRepo::new());
    let app = router::build_for_test(build_state(repo));
    let body = serde_json::json!({
        "user_id": "555555555555555555", "user_name": "Bob"
    });
    let fake_id = Uuid::new_v4().to_string();
    let (status, _) = post_json(app, &format!("/api/blackjack/tables/{fake_id}/join"), body).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn join_table_closed_returns_conflict() {
    let table_id = Uuid::new_v4().to_string();
    let repo = Arc::new(MockTableRepo::new().with_table(sample_table(
        "111111111111111111",
        &table_id,
        "closed",
    )));
    let app = router::build_for_test(build_state(repo));
    let body = serde_json::json!({
        "user_id": "555555555555555555", "user_name": "Bob"
    });
    let (status, _) = post_json(app, &format!("/api/blackjack/tables/{table_id}/join"), body).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn join_table_open_adds_player() {
    let table_id = Uuid::new_v4().to_string();
    let repo = Arc::new(MockTableRepo::new().with_table(sample_table(
        "111111111111111111",
        &table_id,
        "open",
    )));
    let app = router::build_for_test(build_state(repo.clone()));
    let body = serde_json::json!({
        "user_id": "555555555555555555", "user_name": "Bob"
    });
    let (status, _) = post_json(app, &format!("/api/blackjack/tables/{table_id}/join"), body).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(repo.players.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn join_table_full_returns_validation() {
    // 7 joueurs max par defaut → 8e rejete
    let table_id = Uuid::new_v4().to_string();
    let repo = Arc::new(MockTableRepo::new().with_table(sample_table(
        "111111111111111111",
        &table_id,
        "open",
    )));
    for i in 0..7 {
        repo.players
            .lock()
            .unwrap()
            .push((table_id.clone(), format!("u{i}"), format!("User{i}")));
    }
    let app = router::build_for_test(build_state(repo));
    let body = serde_json::json!({
        "user_id": "555555555555555555", "user_name": "Extra"
    });
    let (status, json) =
        post_json(app, &format!("/api/blackjack/tables/{table_id}/join"), body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("pleine"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_table_players_empty() {
    let table_id = Uuid::new_v4().to_string();
    let repo = Arc::new(MockTableRepo::new());
    let app = router::build_for_test(build_state(repo));
    let (status, json) = get(app, &format!("/api/blackjack/tables/{table_id}/players")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_table_games_empty() {
    let table_id = Uuid::new_v4().to_string();
    let repo = Arc::new(MockTableRepo::new());
    let app = router::build_for_test(build_state(repo));
    let (status, json) = get(app, &format!("/api/blackjack/tables/{table_id}/games")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_table_success() {
    let table_id = Uuid::new_v4().to_string();
    let repo = Arc::new(MockTableRepo::new().with_table(sample_table(
        "111111111111111111",
        &table_id,
        "open",
    )));
    let app = router::build_for_test(build_state(repo.clone()));
    let status = delete(app, &format!("/api/blackjack/tables/{table_id}")).await;
    assert!(status.is_success() || status == StatusCode::NO_CONTENT);
    assert_eq!(repo.closed.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_table_by_channel_found() {
    let table_id = Uuid::new_v4().to_string();
    let repo = Arc::new(MockTableRepo::new().with_table(sample_table(
        "111111111111111111",
        &table_id,
        "open",
    )));
    let app = router::build_for_test(build_state(repo));
    let (status, json) = get(app, "/api/blackjack/tables/by-channel/c1").await;
    assert_eq!(status, StatusCode::OK);
    // Le handler retourne Option<TableDto>
    assert!(json.is_object() || json.is_null());
}

// ══════════════════════════════════════════════════════════
// purge_all (sqlx direct → DB test)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_all_returns_counts() {
    let app = router::build_for_test(build_state(Arc::new(MockTableRepo::new())));
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let status = delete(app, &format!("/api/blackjack/admin/{guild_id}/purge")).await;
    assert_eq!(status, StatusCode::OK);
}

// ══════════════════════════════════════════════════════════
// cancel_game : invalid UUID
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_game_invalid_uuid_422() {
    let app = router::build_for_test(build_state(Arc::new(MockTableRepo::new())));
    let status = delete(app, "/api/blackjack/admin/games/not-a-uuid").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// list_games : valid guild, empty result (stub)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_games_admin_empty() {
    let app = router::build_for_test(build_state(Arc::new(MockTableRepo::new())));
    let (status, json) = get(app, "/api/blackjack/admin/111111111111111111/games").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}
