//! Tests d'integration HTTP pour les endpoints coude/bets.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::ports::inbound::coude::manage_bets::ManageCoudeBetsUseCase;
use sentinel_api::ports::inbound::coude::manage_bets::PlaceBetOutcome;
use sentinel_api::ports::inbound::coude::manage_bets::ResolveBetsOutcome;
use sentinel_core::domain::entities::coude::bet::Bet;
use sentinel_core::domain::entities::coude::bet::BetResolutionPlan;
use sentinel_core::domain::entities::coude::bet::NewCoudeBet;
use sentinel_core::domain::entities::coude::bet::RefundSummary;
use sentinel_core::domain::errors::DomainError;
#[derive(Default)]
struct MockBets {
    placed: Mutex<Vec<NewCoudeBet>>,
    resolved: Mutex<Vec<(Uuid, Option<String>)>>,
    refunded: Mutex<Vec<Uuid>>,
    fail_place: Mutex<bool>,
}

#[async_trait]
impl ManageCoudeBetsUseCase for MockBets {
    async fn place(&self, n: NewCoudeBet) -> Result<PlaceBetOutcome, DomainError> {
        if *self.fail_place.lock().unwrap() {
            return Err(DomainError::ValidationError("combat pas en betting".into()));
        }
        self.placed.lock().unwrap().push(n);
        Ok(PlaceBetOutcome {
            taunt_events: vec![],
        })
    }
    async fn list_for_combat(&self, _: Uuid) -> Result<Vec<Bet>, DomainError> {
        Ok(vec![])
    }
    async fn resolve(
        &self,
        combat_id: Uuid,
        winner_id: Option<String>,
    ) -> Result<ResolveBetsOutcome, DomainError> {
        self.resolved.lock().unwrap().push((combat_id, winner_id));
        Ok(ResolveBetsOutcome {
            plan: BetResolutionPlan {
                payouts: vec![],
                fighter_bonus: None,
            },
            taunt_events: vec![],
        })
    }
    async fn refund(&self, combat_id: Uuid) -> Result<RefundSummary, DomainError> {
        self.refunded.lock().unwrap().push(combat_id);
        Ok(RefundSummary {
            refunded_count: 3,
            refunded_total: 600,
        })
    }
}

fn state_with(bets: Arc<MockBets>) -> sentinel_api::adapters::inbound::http::state::AppState {
    let mut s = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    s.coude_bets_uc = bets;
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

// ── place_bet ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn place_bet_success_returns_empty_taunts() {
    let bets = Arc::new(MockBets::default());
    let app = router::build_for_test(state_with(bets.clone()));
    let combat_id = Uuid::new_v4();
    let body = serde_json::json!({
        "combat_id": combat_id.to_string(),
        "bettor_id": "111",
        "bettor_name": "Alice",
        "backed_id": "222",
        "amount": 100,
    });
    let (status, json) = req_json(app, "POST", "/api/coude/999/bets", Some(body)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["taunt_events"].as_array().unwrap().is_empty());
    let placed = bets.placed.lock().unwrap();
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].combat_id, combat_id);
    assert_eq!(placed[0].amount, 100);
    assert_eq!(placed[0].bettor_id, "111");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn place_bet_invalid_combat_id_422() {
    let bets = Arc::new(MockBets::default());
    let app = router::build_for_test(state_with(bets));
    let body = serde_json::json!({
        "combat_id": "not-a-uuid",
        "bettor_id": "1", "bettor_name": "A", "backed_id": "2", "amount": 50
    });
    let (status, json) = req_json(app, "POST", "/api/coude/999/bets", Some(body)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("combat"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn place_bet_domain_error_surfaces_as_422() {
    let bets = Arc::new(MockBets::default());
    *bets.fail_place.lock().unwrap() = true;
    let app = router::build_for_test(state_with(bets));
    let combat_id = Uuid::new_v4();
    let body = serde_json::json!({
        "combat_id": combat_id.to_string(),
        "bettor_id": "1", "bettor_name": "A", "backed_id": "2", "amount": 50
    });
    let (status, _) = req_json(app, "POST", "/api/coude/999/bets", Some(body)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ── get_combat_bets ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_combat_bets_returns_empty_array() {
    let bets = Arc::new(MockBets::default());
    let app = router::build_for_test(state_with(bets));
    let combat_id = Uuid::new_v4();
    let (status, json) = req_json(
        app,
        "GET",
        &format!("/api/coude/combats/{combat_id}/bets"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_combat_bets_invalid_uuid_422() {
    let bets = Arc::new(MockBets::default());
    let app = router::build_for_test(state_with(bets));
    let (status, _) = req_json(app, "GET", "/api/coude/combats/bad-uuid/bets", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ── resolve_bets ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_bets_with_winner_forwards_to_uc() {
    let bets = Arc::new(MockBets::default());
    let app = router::build_for_test(state_with(bets.clone()));
    let combat_id = Uuid::new_v4();
    let body = serde_json::json!({ "winner_id": "111" });
    let (status, json) = req_json(
        app,
        "POST",
        &format!("/api/coude/combats/{combat_id}/resolve-bets"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["results"].as_array().unwrap().is_empty());
    assert!(json["fighter_bonus"].is_null());
    let resolved = bets.resolved.lock().unwrap();
    assert_eq!(resolved[0], (combat_id, Some("111".into())));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_bets_without_winner_treats_as_draw() {
    let bets = Arc::new(MockBets::default());
    let app = router::build_for_test(state_with(bets.clone()));
    let combat_id = Uuid::new_v4();
    let body = serde_json::json!({ "winner_id": null });
    let (status, _) = req_json(
        app,
        "POST",
        &format!("/api/coude/combats/{combat_id}/resolve-bets"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bets.resolved.lock().unwrap()[0].1, None);
}

// ── refund_bets ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refund_bets_returns_summary() {
    let bets = Arc::new(MockBets::default());
    let app = router::build_for_test(state_with(bets.clone()));
    let combat_id = Uuid::new_v4();
    let (status, json) = req_json(
        app,
        "POST",
        &format!("/api/coude/combats/{combat_id}/refund-bets"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["refunded_count"], 3);
    assert_eq!(json["refunded_total"], 600);
    assert_eq!(bets.refunded.lock().unwrap()[0], combat_id);
}
