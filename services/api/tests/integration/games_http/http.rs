//! Tests d'integration HTTP pour les endpoints games.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::outbound::{Game, GamePanel, GameRepository};

use test_helpers::build_test_state_game;

// ══════════════════════════════════════════════════════════
// Mock GameRepository
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockGameRepo {
    games: Mutex<Vec<Game>>,
    panels: Mutex<Vec<GamePanel>>,
    fail_create: Mutex<bool>,
}

impl MockGameRepo {
    fn new() -> Self { Self::default() }
    fn with_game(self, g: Game) -> Self { self.games.lock().unwrap().push(g); self }
}

fn sample_game(guild_id: &str, name: &str) -> Game {
    Game {
        id: Uuid::new_v4().to_string(),
        guild_id: guild_id.into(),
        game_name: name.into(),
        created_by: "444444444444444444".into(),
        created_at: "2024-01-01T00:00:00Z".into(),
        emoji: None,
        category: None,
        role_id: None,
    }
}

#[async_trait]
impl GameRepository for MockGameRepo {
    async fn list(&self, guild_id: &str) -> Result<Vec<Game>, DomainError> {
        Ok(self.games.lock().unwrap().iter()
            .filter(|g| g.guild_id == guild_id).cloned().collect())
    }
    async fn list_by_category(&self, guild_id: &str, category: Option<&str>) -> Result<Vec<Game>, DomainError> {
        Ok(self.games.lock().unwrap().iter()
            .filter(|g| g.guild_id == guild_id)
            .filter(|g| category.is_none_or(|c| g.category.as_deref() == Some(c)))
            .cloned().collect())
    }
    async fn create(&self, guild_id: &str, name: &str, by: &str, emoji: Option<&str>, category: Option<&str>, role_id: Option<&str>) -> Result<Game, DomainError> {
        if *self.fail_create.lock().unwrap() {
            return Err(DomainError::Internal("simulated create failure".into()));
        }
        let g = Game {
            id: Uuid::new_v4().to_string(),
            guild_id: guild_id.into(),
            game_name: name.into(),
            created_by: by.into(),
            created_at: "2024-01-01T00:00:00Z".into(),
            emoji: emoji.map(str::to_string),
            category: category.map(str::to_string),
            role_id: role_id.map(str::to_string),
        };
        self.games.lock().unwrap().push(g.clone());
        Ok(g)
    }
    async fn update(&self, guild_id: &str, game_id: &str, name: Option<&str>, emoji: Option<Option<&str>>, category: Option<Option<&str>>) -> Result<Option<Game>, DomainError> {
        let mut games = self.games.lock().unwrap();
        let g = games.iter_mut()
            .find(|g| g.guild_id == guild_id && g.id == game_id);
        match g {
            Some(g) => {
                if let Some(n) = name { g.game_name = n.into(); }
                if let Some(e) = emoji { g.emoji = e.map(str::to_string); }
                if let Some(c) = category { g.category = c.map(str::to_string); }
                Ok(Some(g.clone()))
            }
            None => Ok(None),
        }
    }
    async fn delete(&self, guild_id: &str, game_id: &str) -> Result<bool, DomainError> {
        let mut games = self.games.lock().unwrap();
        let before = games.len();
        games.retain(|g| !(g.guild_id == guild_id && g.id == game_id));
        Ok(before != games.len())
    }
    async fn find_by_name(&self, guild_id: &str, name: &str) -> Result<Option<Game>, DomainError> {
        Ok(self.games.lock().unwrap().iter()
            .find(|g| g.guild_id == guild_id && g.game_name.eq_ignore_ascii_case(name))
            .cloned())
    }
    async fn set_role_id(&self, guild_id: &str, game_id: &str, role_id: Option<&str>) -> Result<Option<Game>, DomainError> {
        let mut games = self.games.lock().unwrap();
        let g = games.iter_mut()
            .find(|g| g.guild_id == guild_id && g.id == game_id);
        match g {
            Some(g) => { g.role_id = role_id.map(str::to_string); Ok(Some(g.clone())) }
            None => Ok(None),
        }
    }
    async fn save_panel(&self, guild_id: &str, channel_id: &str, message_id: &str, category: Option<&str>) -> Result<GamePanel, DomainError> {
        let p = GamePanel {
            id: Uuid::new_v4().to_string(),
            guild_id: guild_id.into(),
            channel_id: channel_id.into(),
            message_id: message_id.into(),
            category: category.map(str::to_string),
        };
        self.panels.lock().unwrap().push(p.clone());
        Ok(p)
    }
    async fn find_panel_by_message(&self, guild_id: &str, message_id: &str) -> Result<Option<GamePanel>, DomainError> {
        Ok(self.panels.lock().unwrap().iter()
            .find(|p| p.guild_id == guild_id && p.message_id == message_id)
            .cloned())
    }
    async fn list_panels(&self, guild_id: &str) -> Result<Vec<GamePanel>, DomainError> {
        Ok(self.panels.lock().unwrap().iter()
            .filter(|p| p.guild_id == guild_id).cloned().collect())
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_app(repo: Arc<MockGameRepo>) -> axum::Router {
    router::build_for_test(build_test_state_game(repo))
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

async fn send_request(app: axum::Router, req: axum::http::Request<Body>) -> (StatusCode, serde_json::Value) {
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null))
}

// ══════════════════════════════════════════════════════════
// list_games
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_games_empty() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let (status, json) = get(app, "/api/games/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_games_scoped_to_guild() {
    let repo = MockGameRepo::new()
        .with_game(sample_game("111111111111111111", "Valorant"))
        .with_game(sample_game("222222222222222222", "Fortnite"));
    let app = build_app(Arc::new(repo));
    let (status, json) = get(app, "/api/games/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["game_name"], "Valorant");
}

// ══════════════════════════════════════════════════════════
// create_game
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_game_with_provided_role_id_skips_discord() {
    // Avec role_id fourni, aucune creation Discord — le handler utilise tel quel.
    let repo = Arc::new(MockGameRepo::new());
    let app = build_app(repo.clone());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "game_name": "My Game",
        "created_by": "444444444444444444",
        "role_id": "555555555555555555"
    });
    let (status, json) = post_json(app, "/api/games", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["game_name"], "My Game");
    assert_eq!(json["role_id"], "555555555555555555");
    assert_eq!(repo.games.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_game_auto_creates_discord_role_when_absent() {
    // Sans role_id fourni, le handler appelle discord_api.create_role (MockDiscordApi
    // retourne {"id": "r1"}) puis discord_api.edit_role (best-effort, Ok).
    let repo = Arc::new(MockGameRepo::new());
    let app = build_app(repo.clone());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "game_name": "Auto Role Game",
        "created_by": "444444444444444444"
    });
    let (status, json) = post_json(app, "/api/games", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["role_id"], "r1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_game_rejects_empty_name() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "game_name": "",
        "created_by": "444444444444444444",
        "role_id": "r1"
    });
    let (status, json) = post_json(app, "/api/games", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("vide"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_game_rejects_whitespace_only_name() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "game_name": "    ",
        "created_by": "444444444444444444",
        "role_id": "r1"
    });
    let (status, _) = post_json(app, "/api/games", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_game_rejects_name_over_100_chars() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "game_name": "a".repeat(101),
        "created_by": "444444444444444444",
        "role_id": "r1"
    });
    let (status, json) = post_json(app, "/api/games", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("100"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_game_trims_whitespace() {
    let repo = Arc::new(MockGameRepo::new());
    let app = build_app(repo.clone());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "game_name": "  Trimmed  ",
        "created_by": "444444444444444444",
        "role_id": "r1"
    });
    let (status, json) = post_json(app, "/api/games", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["game_name"], "Trimmed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_game_empty_emoji_becomes_none() {
    let repo = Arc::new(MockGameRepo::new());
    let app = build_app(repo.clone());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "game_name": "X",
        "created_by": "444444444444444444",
        "role_id": "r1",
        "emoji": "   ",
        "category": "fps"
    });
    let (status, json) = post_json(app, "/api/games", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["emoji"], serde_json::Value::Null);
    assert_eq!(json["category"], "fps");
}

// ══════════════════════════════════════════════════════════
// update_game
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_game_rename_and_category() {
    let game = sample_game("111111111111111111", "Old");
    let id = game.id.clone();
    let repo = Arc::new(MockGameRepo::new().with_game(game));
    let app = build_app(repo);
    let body = serde_json::json!({
        "game_name": "New Name",
        "category": "fps"
    });
    let (status, json) = patch_json(app, &format!("/api/games/111111111111111111/{id}"), body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["game_name"], "New Name");
    assert_eq!(json["category"], "fps");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_game_not_found_returns_404() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let body = serde_json::json!({"game_name": "X"});
    let fake_id = Uuid::new_v4().to_string();
    let (status, _) = patch_json(app, &format!("/api/games/111111111111111111/{fake_id}"), body).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_game_name_over_100_chars_422() {
    let game = sample_game("111111111111111111", "Old");
    let id = game.id.clone();
    let repo = Arc::new(MockGameRepo::new().with_game(game));
    let app = build_app(repo);
    let body = serde_json::json!({"game_name": "a".repeat(101)});
    let (status, _) = patch_json(app, &format!("/api/games/111111111111111111/{id}"), body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_game_with_rbac_viewer_forbidden() {
    use sentinel_api::adapters::inbound::http::middleware::rbac::Role;
    let game = sample_game("111111111111111111", "Old");
    let id = game.id.clone();
    let repo = Arc::new(MockGameRepo::new().with_game(game));
    let app = build_app(repo);
    let req = test_helpers::request_with_rbac(
        "PATCH", &format!("/api/games/111111111111111111/{id}"),
        "555555555555555555", Some(Role::Viewer), Some("111111111111111111".into()),
        Some(serde_json::json!({"game_name": "X"})),
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ══════════════════════════════════════════════════════════
// delete_game
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_game_success_204() {
    let game = sample_game("111111111111111111", "X");
    let id = game.id.clone();
    let repo = Arc::new(MockGameRepo::new().with_game(game));
    let app = build_app(repo.clone());
    let status = delete(app, &format!("/api/games/111111111111111111/{id}")).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(repo.games.lock().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_game_not_found_404() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let fake_id = Uuid::new_v4().to_string();
    let status = delete(app, &format!("/api/games/111111111111111111/{fake_id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_game_with_rbac_viewer_forbidden() {
    use sentinel_api::adapters::inbound::http::middleware::rbac::Role;
    let app = build_app(Arc::new(MockGameRepo::new()));
    let fake_id = Uuid::new_v4().to_string();
    let req = test_helpers::request_with_rbac(
        "DELETE", &format!("/api/games/111111111111111111/{fake_id}"),
        "555555555555555555", Some(Role::Viewer), None, None,
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ══════════════════════════════════════════════════════════
// set_role_id / get_game_by_name / list_by_category
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_role_id_with_value() {
    let game = sample_game("111111111111111111", "X");
    let id = game.id.clone();
    let repo = Arc::new(MockGameRepo::new().with_game(game));
    let app = build_app(repo);
    let body = serde_json::json!({"role_id": "999999999999999999"});
    let (status, json) = patch_json(app, &format!("/api/games/111111111111111111/{id}/role"), body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["role_id"], "999999999999999999");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_role_id_null_clears_binding() {
    let mut g = sample_game("111111111111111111", "X");
    g.role_id = Some("999".into());
    let id = g.id.clone();
    let repo = Arc::new(MockGameRepo::new().with_game(g));
    let app = build_app(repo);
    let body = serde_json::json!({"role_id": null});
    let (status, json) = patch_json(app, &format!("/api/games/111111111111111111/{id}/role"), body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["role_id"], serde_json::Value::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_role_id_empty_string_clears_binding() {
    // Empty string apres trim devient None (normalize_optional_tag)
    let mut g = sample_game("111111111111111111", "X");
    g.role_id = Some("999".into());
    let id = g.id.clone();
    let repo = Arc::new(MockGameRepo::new().with_game(g));
    let app = build_app(repo);
    let body = serde_json::json!({"role_id": "   "});
    let (status, json) = patch_json(app, &format!("/api/games/111111111111111111/{id}/role"), body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["role_id"], serde_json::Value::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_game_by_name_found() {
    let repo = Arc::new(MockGameRepo::new().with_game(sample_game("111111111111111111", "Valorant")));
    let app = build_app(repo);
    let (status, json) = get(app, "/api/games/111111111111111111/by-name/valorant").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["game_name"], "Valorant");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_game_by_name_not_found_returns_null() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let (status, json) = get(app, "/api/games/111111111111111111/by-name/unknown").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, serde_json::Value::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_by_category_filter() {
    let mut g1 = sample_game("111111111111111111", "Valorant");
    g1.category = Some("fps".into());
    let mut g2 = sample_game("111111111111111111", "LoL");
    g2.category = Some("moba".into());
    let repo = Arc::new(MockGameRepo::new().with_game(g1).with_game(g2));
    let app = build_app(repo);
    let (status, json) = get(app, "/api/games/111111111111111111/by-category?category=fps").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["game_name"], "Valorant");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_by_category_empty_param_returns_all() {
    let repo = Arc::new(MockGameRepo::new()
        .with_game(sample_game("111111111111111111", "A"))
        .with_game(sample_game("111111111111111111", "B")));
    let app = build_app(repo);
    let (status, json) = get(app, "/api/games/111111111111111111/by-category?category=").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 2);
}

// ══════════════════════════════════════════════════════════
// Panels
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_panel_success() {
    let repo = Arc::new(MockGameRepo::new());
    let app = build_app(repo.clone());
    let body = serde_json::json!({
        "channel_id": "c1",
        "message_id": "m1",
        "category": "fps"
    });
    let (status, json) = post_json(app, "/api/games/111111111111111111/panels", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["channel_id"], "c1");
    assert_eq!(json["category"], "fps");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_panel_empty_category_becomes_none() {
    let repo = Arc::new(MockGameRepo::new());
    let app = build_app(repo);
    let body = serde_json::json!({
        "channel_id": "c1",
        "message_id": "m1",
        "category": "   "
    });
    let (status, json) = post_json(app, "/api/games/111111111111111111/panels", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["category"], serde_json::Value::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_panels_empty() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let (status, json) = get(app, "/api/games/111111111111111111/panels").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_panel_by_message_not_found_returns_null() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let (status, json) = get(app, "/api/games/111111111111111111/panels/by-message/m-unknown").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, serde_json::Value::Null);
}

// ══════════════════════════════════════════════════════════
// upload_emoji (multipart)
// ══════════════════════════════════════════════════════════

fn multipart_body(parts: &[(&str, &str, Option<&str>, &[u8])]) -> (String, Vec<u8>) {
    let boundary = "----sentinel-test-boundary";
    let mut body: Vec<u8> = Vec::new();
    for (name, value_or_filename, content_type, data) in parts {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        match content_type {
            Some(ct) => {
                body.extend_from_slice(format!(
                    "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
                    name, value_or_filename, ct,
                ).as_bytes());
                body.extend_from_slice(data);
            }
            None => {
                body.extend_from_slice(format!(
                    "Content-Disposition: form-data; name=\"{}\"\r\n\r\n",
                    name,
                ).as_bytes());
                body.extend_from_slice(value_or_filename.as_bytes());
            }
        }
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

async fn post_multipart(app: axum::Router, uri: &str, ct: String, body: Vec<u8>) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("POST").uri(uri)
        .header("content-type", ct)
        .body(Body::from(body)).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upload_emoji_success_with_mock_discord() {
    // MockDiscordApi.upload_emoji retourne ("emoji_id", "emoji_name", false).
    // On verifie que le handler format correctement la syntaxe Discord.
    let app = build_app(Arc::new(MockGameRepo::new()));
    let (ct, body) = multipart_body(&[
        ("name", "Cool Game!", None, &[]),
        ("image", "cool.png", Some("image/png"), b"fake-png-bytes"),
    ]);
    let (status, json) = post_multipart(app, "/api/games/111111111111111111/upload-emoji", ct, body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["emoji_id"], "emoji_id");
    assert_eq!(json["animated"], false);
    assert_eq!(json["emoji"], "<:emoji_name:emoji_id>");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upload_emoji_missing_name_422() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let (ct, body) = multipart_body(&[
        ("image", "x.png", Some("image/png"), b"xx"),
    ]);
    let (status, json) = post_multipart(app, "/api/games/111111111111111111/upload-emoji", ct, body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("name"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upload_emoji_missing_image_422() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let (ct, body) = multipart_body(&[
        ("name", "n", None, &[]),
    ]);
    let (status, json) = post_multipart(app, "/api/games/111111111111111111/upload-emoji", ct, body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("image"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upload_emoji_oversized_422() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let oversized = vec![0u8; 300 * 1024];
    let (ct, body) = multipart_body(&[
        ("name", "big", None, &[]),
        ("image", "big.png", Some("image/png"), &oversized),
    ]);
    let (status, json) = post_multipart(app, "/api/games/111111111111111111/upload-emoji", ct, body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("256"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upload_emoji_unsupported_mime_422() {
    let app = build_app(Arc::new(MockGameRepo::new()));
    let (ct, body) = multipart_body(&[
        ("name", "n", None, &[]),
        ("image", "n.svg", Some("image/svg+xml"), b"<svg/>"),
    ]);
    let (status, json) = post_multipart(app, "/api/games/111111111111111111/upload-emoji", ct, body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("non supporte"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upload_emoji_with_rbac_viewer_forbidden() {
    use sentinel_api::adapters::inbound::http::middleware::rbac::{Role, RoleContext};
    let (ct, body) = multipart_body(&[
        ("name", "n", None, &[]),
        ("image", "n.png", Some("image/png"), b"xx"),
    ]);
    let app = build_app(Arc::new(MockGameRepo::new()));
    let mut req = Request::builder().method("POST")
        .uri("/api/games/111111111111111111/upload-emoji")
        .header("content-type", ct)
        .body(Body::from(body)).unwrap();
    req.extensions_mut().insert(RoleContext {
        discord_user_id: "444444444444444444".into(),
        role: Some(Role::Viewer),
        guild_id: None,
    });
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
