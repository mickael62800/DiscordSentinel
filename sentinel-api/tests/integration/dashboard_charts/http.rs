//! Tests d'integration HTTP pour GET /api/charts/activity.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use chrono::NaiveDate;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_core::domain::entities::community::daily_activity::DailyActivity;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::community::daily_activity_repository::DailyActivityRepository;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockDailyActivityRepo {
    items: Mutex<Vec<DailyActivity>>,
    last_days: Mutex<Option<i32>>,
    last_guild: Mutex<Option<String>>,
}

impl MockDailyActivityRepo {
    fn new() -> Self {
        Self::default()
    }
    fn with(self, a: DailyActivity) -> Self {
        self.items.lock().unwrap().push(a);
        self
    }
}

#[async_trait]
impl DailyActivityRepository for MockDailyActivityRepo {
    async fn get_activity(
        &self,
        guild_id: Option<&str>,
        days: i32,
    ) -> Result<Vec<DailyActivity>, DomainError> {
        *self.last_days.lock().unwrap() = Some(days);
        *self.last_guild.lock().unwrap() = guild_id.map(str::to_string);
        let items = self.items.lock().unwrap();
        let matching: Vec<DailyActivity> = items
            .iter()
            .filter(|a| guild_id.is_none_or(|g| a.guild_id.as_str() == g))
            .cloned()
            .collect();
        Ok(matching)
    }
    async fn record_daily_snapshot(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

fn sample_day(guild_id: &str, day: NaiveDate, messages: i64) -> DailyActivity {
    DailyActivity {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        day,
        messages,
        voice_minutes: 120,
        active_members: 10,
        new_members: 1,
        leaves: 0,
        infractions: 0,
        warns: 0,
        mutes: 0,
        bans: 0,
    }
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

// ══════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn activity_trend_defaults_30_days() {
    let repo = Arc::new(MockDailyActivityRepo::new());
    let app = router::build_for_test(test_helpers::build_test_state_daily_activity(repo.clone()));
    let (status, _) = get(app, "/api/charts/activity").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(*repo.last_days.lock().unwrap(), Some(30));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn activity_trend_caps_days_at_90() {
    let repo = Arc::new(MockDailyActivityRepo::new());
    let app = router::build_for_test(test_helpers::build_test_state_daily_activity(repo.clone()));
    let (status, _) = get(app, "/api/charts/activity?days=365").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(*repo.last_days.lock().unwrap(), Some(90));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn activity_trend_floors_days_at_1() {
    let repo = Arc::new(MockDailyActivityRepo::new());
    let app = router::build_for_test(test_helpers::build_test_state_daily_activity(repo.clone()));
    let (status, _) = get(app, "/api/charts/activity?days=0").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(*repo.last_days.lock().unwrap(), Some(1));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn activity_trend_forwards_guild_filter() {
    let repo = Arc::new(MockDailyActivityRepo::new());
    let app = router::build_for_test(test_helpers::build_test_state_daily_activity(repo.clone()));
    let (status, _) = get(
        app,
        "/api/charts/activity?guild_id=111111111111111111&days=7",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        repo.last_guild.lock().unwrap().as_deref(),
        Some("111111111111111111")
    );
    assert_eq!(*repo.last_days.lock().unwrap(), Some(7));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn activity_trend_returns_dtos() {
    let day = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let repo = MockDailyActivityRepo::new()
        .with(sample_day("111111111111111111", day, 100))
        .with(sample_day(
            "111111111111111111",
            day.succ_opt().unwrap(),
            150,
        ));
    let app = router::build_for_test(test_helpers::build_test_state_daily_activity(Arc::new(
        repo,
    )));
    let (status, json) = get(
        app,
        "/api/charts/activity?guild_id=111111111111111111&days=30",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["messages"], 100);
}
