//! Tests d'integration HTTP pour POST /api/exports/jobs + GET /api/exports/jobs/{id}.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;

async fn json_req(app: axum::Router, method: &str, uri: &str, body: Option<serde_json::Value>)
    -> (StatusCode, serde_json::Value)
{
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
    (s, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

fn state() -> sentinel_api::adapters::inbound::http::state::AppState {
    test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels))
}

fn body(job_type: &str, format: &str) -> serde_json::Value {
    serde_json::json!({
        "guild_id": "111111111111111111",
        "requested_by": "222222222222222222",
        "job_type": job_type,
        "format": format,
        "filters": {}
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_export_rejects_invalid_guild_id() {
    let app = router::build_for_test(state());
    let mut b = body("infractions", "csv");
    b["guild_id"] = serde_json::json!("bad");
    let (status, _) = json_req(app, "POST", "/api/exports/jobs", Some(b)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_export_rejects_invalid_job_type() {
    let app = router::build_for_test(state());
    let (status, json) = json_req(app, "POST", "/api/exports/jobs", Some(body("unknown", "csv"))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("job_type"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_export_rejects_invalid_format() {
    let app = router::build_for_test(state());
    let (status, json) = json_req(app, "POST", "/api/exports/jobs", Some(body("infractions", "xml"))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("format"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_export_without_rbac_passes_through() {
    // Pas d'Extension<RoleContext> -> check_role_for_guild laisse passer (bot/internal).
    let app = router::build_for_test(state());
    let (status, json) = json_req(app, "POST", "/api/exports/jobs", Some(body("infractions", "csv"))).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(json["status"], "pending");
    assert!(Uuid::parse_str(json["job_id"].as_str().unwrap()).is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_export_viewer_forbidden() {
    use sentinel_api::domain::enums::system::role::Role;
    let app = router::build_for_test(state());
    let req = test_helpers::request_with_rbac(
        "POST", "/api/exports/jobs",
        "333333333333333333", Some(Role::Viewer),
        Some("111111111111111111".into()),
        Some(body("infractions", "csv")),
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_export_invalid_uuid_422() {
    let app = router::build_for_test(state());
    let (status, _) = json_req(app, "GET", "/api/exports/jobs/not-a-uuid", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_export_unknown_404() {
    let app = router::build_for_test(state());
    let (status, _) = json_req(app, "GET", &format!("/api/exports/jobs/{}", Uuid::new_v4()), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_then_get_export_roundtrip() {
    let app = router::build_for_test(state());
    let (s1, j1) = json_req(app.clone(), "POST", "/api/exports/jobs",
        Some(body("audit_logs", "json"))).await;
    assert_eq!(s1, StatusCode::ACCEPTED);
    let job_id = j1["job_id"].as_str().unwrap();
    let (s2, j2) = json_req(app, "GET", &format!("/api/exports/jobs/{job_id}"), None).await;
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(j2["job_type"], "audit_logs");
    assert_eq!(j2["format"], "json");
}
