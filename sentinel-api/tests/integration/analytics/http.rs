//! Tests d'integration HTTP pour les endpoints analytics.

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

use sentinel_api::adapters::inbound::http::router;
use sentinel_core::domain::entities::system::analytics::ActionDistribution;
use sentinel_core::domain::entities::system::analytics::HourlyActivity;
use sentinel_core::domain::entities::system::analytics::ModerationTrend;
use sentinel_core::domain::entities::system::analytics::PeakActivity;
use sentinel_core::domain::entities::system::analytics::TopInfractor;
use sentinel_core::domain::errors::DomainError;
use sentinel_api::ports::outbound::audit::analytics_repository::AnalyticsRepository;

use test_helpers::build_test_state_analytics;

// ══════════════════════════════════════════════════════════
// Mock
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockAnalyticsRepo {
    calls: Mutex<Vec<String>>,
}

impl MockAnalyticsRepo {
    fn new() -> Self { Self::default() }
}

#[async_trait]
impl AnalyticsRepository for MockAnalyticsRepo {
    async fn get_heatmap(&self, _: Option<&str>, _: i32) -> Result<Vec<HourlyActivity>, DomainError> {
        self.calls.lock().unwrap().push("heatmap".into());
        Ok(vec![HourlyActivity { hour: 10, day_of_week: 1, messages: 50, infractions: 2 }])
    }
    async fn get_action_distribution(&self, _: Option<&str>, _: i32) -> Result<Vec<ActionDistribution>, DomainError> {
        self.calls.lock().unwrap().push("actions".into());
        Ok(vec![ActionDistribution { action: "warn".into(), count: 10, percentage: 50.0 }])
    }
    async fn get_top_infractors(&self, _: Option<&str>, _: i32, _: i64) -> Result<Vec<TopInfractor>, DomainError> {
        self.calls.lock().unwrap().push("infractors".into());
        Ok(vec![TopInfractor {
            user_id: "u1".into(), username: "alice".into(),
            total_infractions: 5, warns: 3, deletes: 1, mutes: 1, bans: 0,
        }])
    }
    async fn get_moderation_trend(&self, _: Option<&str>, _: i32) -> Result<Vec<ModerationTrend>, DomainError> {
        self.calls.lock().unwrap().push("trend".into());
        Ok(vec![ModerationTrend {
            day: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            total: 10, warns: 5, deletes: 2, mutes: 2, bans: 1,
        }])
    }
    async fn get_peak_hours(&self, _: Option<&str>, _: i32) -> Result<Vec<PeakActivity>, DomainError> {
        self.calls.lock().unwrap().push("peaks".into());
        Ok(vec![PeakActivity { hour: 20, avg_messages: 100.0, avg_infractions: 5.0 }])
    }
    async fn record_hourly(&self, _: &str, _: i16, _: i64, _: i32) -> Result<(), DomainError> { Ok(()) }
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

fn build_app(repo: Arc<MockAnalyticsRepo>) -> axum::Router {
    router::build_for_test(build_test_state_analytics(repo))
}

// ══════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_analytics_aggregates_five_sources() {
    let app = build_app(Arc::new(MockAnalyticsRepo::new()));
    let (status, json) = get(app, "/api/analytics").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["heatmap"].as_array().unwrap().len() >= 1);
    assert!(json["action_distribution"].as_array().unwrap().len() >= 1);
    assert!(json["top_infractors"].as_array().unwrap().len() >= 1);
    assert!(json["moderation_trend"].as_array().unwrap().len() >= 1);
    assert!(json["peak_hours"].as_array().unwrap().len() >= 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heatmap_endpoint_returns_dtos() {
    let app = build_app(Arc::new(MockAnalyticsRepo::new()));
    let (status, json) = get(app, "/api/analytics/heatmap").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr[0]["hour"], 10);
    assert_eq!(arr[0]["messages"], 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn action_distribution_endpoint() {
    let app = build_app(Arc::new(MockAnalyticsRepo::new()));
    let (status, json) = get(app, "/api/analytics/actions").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr[0]["action"], "warn");
    assert_eq!(arr[0]["count"], 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn top_infractors_endpoint() {
    let app = build_app(Arc::new(MockAnalyticsRepo::new()));
    let (status, json) = get(app, "/api/analytics/top-infractors").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr[0]["user_id"], "u1");
    assert_eq!(arr[0]["total_infractions"], 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn moderation_trend_endpoint() {
    let app = build_app(Arc::new(MockAnalyticsRepo::new()));
    let (status, json) = get(app, "/api/analytics/moderation-trend").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr[0]["total"], 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peak_hours_endpoint() {
    let app = build_app(Arc::new(MockAnalyticsRepo::new()));
    let (status, json) = get(app, "/api/analytics/peak-hours").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr[0]["hour"], 20);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analytics_params_accept_days_and_guild_and_limit() {
    // guild_id unique → cache Redis miss → repo.get_top_infractors appele.
    let repo = Arc::new(MockAnalyticsRepo::new());
    let app = build_app(repo.clone());
    let uniq = uuid::Uuid::new_v4().to_string().replace('-', "");
    let (status, _) = get(app, &format!("/api/analytics/top-infractors?guild_id={uniq}&days=14&limit=5")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(repo.calls.lock().unwrap().contains(&"infractors".to_string()));
}
