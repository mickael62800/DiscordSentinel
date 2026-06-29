//! Tests d'integration HTTP pour les endpoints coude/social.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use chrono::DateTime;
use chrono::Duration;
use chrono::TimeZone;
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::ports::inbound::coude::manage_social::ManageCoudeSocialUseCase;
use sentinel_core::domain::entities::coude::social::DailyChaosOutcome;
use sentinel_core::domain::entities::coude::social::Event;
use sentinel_core::domain::entities::coude::social::LeaderboardCategory;
use sentinel_core::domain::entities::coude::social::LeaderboardEntry;
use sentinel_core::domain::entities::coude::social::NewDailyChaos;
use sentinel_core::domain::entities::coude::social::Season;
use sentinel_core::domain::errors::DomainError;

#[derive(Default)]
struct MockSocial {
    cooldown_hits: Mutex<Vec<(String, String, String)>>,
    cooldown_set: Mutex<Vec<(String, String, String, i64)>>,
    leaderboard_calls: Mutex<Vec<(String, LeaderboardCategory, i64)>>,
    chaos_logged: Mutex<Vec<NewDailyChaos>>,
    fixed_cooldown: Mutex<Option<DateTime<Utc>>>,
}

#[async_trait]
impl ManageCoudeSocialUseCase for MockSocial {
    async fn check_cooldown(
        &self,
        g: &str,
        u: &str,
        a: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        self.cooldown_hits
            .lock()
            .unwrap()
            .push((g.into(), u.into(), a.into()));
        Ok(*self.fixed_cooldown.lock().unwrap())
    }
    async fn set_cooldown(&self, g: &str, u: &str, a: &str, d: i64) -> Result<(), DomainError> {
        self.cooldown_set
            .lock()
            .unwrap()
            .push((g.into(), u.into(), a.into(), d));
        Ok(())
    }
    async fn leaderboard(
        &self,
        g: &str,
        cat: LeaderboardCategory,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, DomainError> {
        self.leaderboard_calls
            .lock()
            .unwrap()
            .push((g.into(), cat, limit));
        Ok(vec![
            LeaderboardEntry {
                user_id: "1".into(),
                username: "Alice".into(),
                value: 1000,
            },
            LeaderboardEntry {
                user_id: "2".into(),
                username: "Bob".into(),
                value: 500,
            },
        ])
    }
    async fn list_active_events(&self, _: &str) -> Result<Vec<Event>, DomainError> {
        Ok(vec![])
    }
    async fn log_daily_chaos(&self, c: NewDailyChaos) -> Result<(), DomainError> {
        self.chaos_logged.lock().unwrap().push(c);
        Ok(())
    }
    async fn current_season(&self, _: &str) -> Result<Season, DomainError> {
        let started = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        Ok(Season {
            season_number: 3,
            started_at: started,
            ends_at: started + Duration::days(90),
            days_remaining: 45,
        })
    }
    async fn trigger_daily_chaos(&self, _: &str) -> Result<Option<DailyChaosOutcome>, DomainError> {
        Ok(None)
    }
}

fn state_with(uc: Arc<MockSocial>) -> sentinel_api::adapters::inbound::http::state::AppState {
    let mut s = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    s.coude_social_uc = uc;
    s
}

async fn req_json(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut b = Request::builder().method(method).uri(uri);
    let body_payload = match body {
        Some(v) => {
            b = b.header("content-type", "application/json");
            Body::from(serde_json::to_string(&v).unwrap())
        }
        None => Body::empty(),
    };
    let req = b.body(body_payload).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

// ── Cooldowns ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn check_cooldown_none_returns_null() {
    let uc = Arc::new(MockSocial::default());
    let app = router::build_for_test(state_with(uc.clone()));
    let (status, json) = req_json(app, "GET", "/api/coude/999/cooldown/111/steal", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["expires_at"].is_null());
    let hits = uc.cooldown_hits.lock().unwrap();
    assert_eq!(hits[0], ("999".into(), "111".into(), "steal".into()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn check_cooldown_some_returns_rfc3339() {
    let uc = Arc::new(MockSocial::default());
    let ts = Utc.with_ymd_and_hms(2026, 6, 15, 12, 0, 0).unwrap();
    *uc.fixed_cooldown.lock().unwrap() = Some(ts);
    let app = router::build_for_test(state_with(uc));
    let (status, json) = req_json(app, "GET", "/api/coude/999/cooldown/111/steal", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["expires_at"], "2026-06-15T12:00:00+00:00");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_cooldown_forwards_duration() {
    let uc = Arc::new(MockSocial::default());
    let app = router::build_for_test(state_with(uc.clone()));
    let body = serde_json::json!({ "duration_secs": 3600 });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/coude/999/cooldown/111/steal")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let set = uc.cooldown_set.lock().unwrap();
    assert_eq!(set[0], ("999".into(), "111".into(), "steal".into(), 3600));
}

// ── Leaderboard ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_richest_returns_entries() {
    let uc = Arc::new(MockSocial::default());
    let app = router::build_for_test(state_with(uc.clone()));
    let (status, json) = req_json(app, "GET", "/api/coude/999/leaderboard/richest", None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["username"], "Alice");
    assert_eq!(arr[0]["value"], 1000);
    let calls = uc.leaderboard_calls.lock().unwrap();
    assert_eq!(calls[0].1, LeaderboardCategory::Richest);
    // Default limit = DEFAULT_COUDE_SOCIAL_LEADERBOARD_LIMIT (10)
    assert_eq!(calls[0].2, 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_custom_limit_passed_through() {
    let uc = Arc::new(MockSocial::default());
    let app = router::build_for_test(state_with(uc.clone()));
    let (status, _) = req_json(
        app,
        "GET",
        "/api/coude/999/leaderboard/chaos?limit=25",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(uc.leaderboard_calls.lock().unwrap()[0].2, 25);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_invalid_category_422() {
    let uc = Arc::new(MockSocial::default());
    let app = router::build_for_test(state_with(uc));
    let (status, json) = req_json(app, "GET", "/api/coude/999/leaderboard/unknown", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("Categorie"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_all_valid_categories_accepted() {
    for cat in ["richest", "thieves", "cowards", "chaos", "level"] {
        let uc = Arc::new(MockSocial::default());
        let app = router::build_for_test(state_with(uc));
        let (status, _) = req_json(
            app,
            "GET",
            &format!("/api/coude/999/leaderboard/{cat}"),
            None,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "categorie {cat} devrait etre acceptee"
        );
    }
}

// ── Daily chaos ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_daily_chaos_forwards_payload() {
    let uc = Arc::new(MockSocial::default());
    let app = router::build_for_test(state_with(uc.clone()));
    let body = serde_json::json!({
        "loser_id": "111", "loser_name": "L",
        "winner_id": "222", "winner_name": "W",
        "amount": 500
    });
    let (status, _) = req_json(app, "POST", "/api/coude/999/daily-chaos", Some(body)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let logged = uc.chaos_logged.lock().unwrap();
    assert_eq!(logged[0].guild_id, "999");
    assert_eq!(logged[0].loser_id, "111");
    assert_eq!(logged[0].amount, 500);
}

// ── Events ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_active_events_returns_empty() {
    let uc = Arc::new(MockSocial::default());
    let app = router::build_for_test(state_with(uc));
    let (status, json) = req_json(app, "GET", "/api/coude/999/events", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().is_empty());
}

// ── Season ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_current_season_returns_dto() {
    let uc = Arc::new(MockSocial::default());
    let app = router::build_for_test(state_with(uc));
    let (status, json) = req_json(app, "GET", "/api/coude/999/season/current", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["season_number"], 3);
    assert_eq!(json["days_remaining"], 45);
    assert!(json["started_at"]
        .as_str()
        .unwrap()
        .starts_with("2026-01-01"));
}
