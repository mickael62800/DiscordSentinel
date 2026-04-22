//! Tests d'integration HTTP pour les endpoints coude/economy.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::domain::entities::TauntEvent;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::manage_coude_economy::StealOutcome;
use sentinel_api::ports::inbound::ManageCoudeEconomyUseCase;

#[derive(Default)]
struct MockEconomy {
    transfers: Mutex<Vec<(String, String, String, i64)>>,
    steals: Mutex<Vec<(String, String, String, i64)>>,
    steal_penalty: Mutex<Vec<(String, String, i64)>>,
    casino_wins: Mutex<Vec<(String, String, i64)>>,
    casino_losses: Mutex<Vec<(String, String, i64)>>,
    casino_faillites: Mutex<Vec<(String, String)>>,
    counts_casino: Mutex<i64>,
    sum_gains: Mutex<i64>,
    counts_steal: Mutex<i64>,
    steal_stolen: Mutex<i64>,
}

#[async_trait]
impl ManageCoudeEconomyUseCase for MockEconomy {
    async fn transfer(&self, g: &str, from: &str, to: &str, amt: i64)
        -> Result<Vec<TauntEvent>, DomainError>
    {
        self.transfers.lock().unwrap().push((g.into(), from.into(), to.into(), amt));
        Ok(vec![])
    }
    async fn steal(&self, g: &str, t: &str, v: &str, amt: i64)
        -> Result<StealOutcome, DomainError>
    {
        self.steals.lock().unwrap().push((g.into(), t.into(), v.into(), amt));
        Ok(StealOutcome { stolen: *self.steal_stolen.lock().unwrap(), taunt_events: vec![] })
    }
    async fn steal_fail_penalty(&self, g: &str, t: &str, amt: i64)
        -> Result<(i64, Vec<TauntEvent>), DomainError>
    {
        self.steal_penalty.lock().unwrap().push((g.into(), t.into(), amt));
        Ok((amt / 2, vec![]))
    }
    async fn record_casino_win(&self, g: &str, u: &str, gain: i64) -> Result<(), DomainError> {
        self.casino_wins.lock().unwrap().push((g.into(), u.into(), gain));
        Ok(())
    }
    async fn record_casino_loss(&self, g: &str, u: &str, lost: i64) -> Result<(), DomainError> {
        self.casino_losses.lock().unwrap().push((g.into(), u.into(), lost));
        Ok(())
    }
    async fn record_casino_faillite(&self, g: &str, u: &str) -> Result<i64, DomainError> {
        self.casino_faillites.lock().unwrap().push((g.into(), u.into()));
        Ok(1234)
    }
    async fn count_casino_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(*self.counts_casino.lock().unwrap())
    }
    async fn sum_casino_gains_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(*self.sum_gains.lock().unwrap())
    }
    async fn count_steal_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(*self.counts_steal.lock().unwrap())
    }
}

fn state_with(m: Arc<MockEconomy>) -> sentinel_api::adapters::inbound::http::state::AppState {
    let mut s = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    s.coude_economy_uc = m;
    s
}

async fn req_json(app: axum::Router, method: &str, uri: &str, body: Option<serde_json::Value>)
    -> (StatusCode, serde_json::Value)
{
    let mut b = Request::builder().method(method).uri(uri);
    let body_payload = match body {
        Some(v) => { b = b.header("content-type", "application/json"); Body::from(serde_json::to_string(&v).unwrap()) }
        None => Body::empty(),
    };
    let req = b.body(body_payload).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

// ── Transfer ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_coins_forwards_and_returns_taunts() {
    let m = Arc::new(MockEconomy::default());
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({ "from_id": "111", "to_id": "222", "amount": 500 });
    let (s, j) = req_json(app, "POST", "/api/coude/999/transfer", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["taunt_events"].as_array().unwrap().is_empty());
    assert_eq!(m.transfers.lock().unwrap()[0], ("999".into(), "111".into(), "222".into(), 500));
}

// ── Steal ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_steal_returns_stolen_amount() {
    let m = Arc::new(MockEconomy::default());
    *m.steal_stolen.lock().unwrap() = 250;
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({ "thief_id": "111", "victim_id": "222", "amount": 300 });
    let (s, j) = req_json(app, "POST", "/api/coude/999/steal", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["stolen"], 250);
    assert_eq!(m.steals.lock().unwrap()[0].3, 300);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn steal_fail_penalty_returns_half_amount() {
    let m = Arc::new(MockEconomy::default());
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({ "thief_id": "111", "amount": 400 });
    let (s, j) = req_json(app, "POST", "/api/coude/999/steal-fail-penalty", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["lost"], 200);
    assert_eq!(m.steal_penalty.lock().unwrap()[0], ("999".into(), "111".into(), 400));
}

// ── Casino ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_casino_win_forwards_gain() {
    let m = Arc::new(MockEconomy::default());
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({ "gain": 100 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/casino-win", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(m.casino_wins.lock().unwrap()[0].2, 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_casino_loss_forwards_lost() {
    let m = Arc::new(MockEconomy::default());
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({ "lost": 50 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/casino-loss", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(m.casino_losses.lock().unwrap()[0].2, 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_casino_faillite_returns_total_lost() {
    let m = Arc::new(MockEconomy::default());
    let app = router::build_for_test(state_with(m.clone()));
    let (s, j) = req_json(app, "POST", "/api/coude/999/players/111/casino-faillite", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["total_lost"], 1234);
    assert_eq!(m.casino_faillites.lock().unwrap()[0], ("999".into(), "111".into()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn count_casino_today_returns_count() {
    let m = Arc::new(MockEconomy::default());
    *m.counts_casino.lock().unwrap() = 7;
    let app = router::build_for_test(state_with(m));
    let (_, j) = req_json(app, "GET", "/api/coude/999/players/111/casino-today", None).await;
    assert_eq!(j["count"], 7);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sum_casino_gains_today_returns_total() {
    let m = Arc::new(MockEconomy::default());
    *m.sum_gains.lock().unwrap() = 1500;
    let app = router::build_for_test(state_with(m));
    let (_, j) = req_json(app, "GET", "/api/coude/999/players/111/casino-gains-today", None).await;
    assert_eq!(j["total"], 1500);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn count_steal_today_returns_count() {
    let m = Arc::new(MockEconomy::default());
    *m.counts_steal.lock().unwrap() = 3;
    let app = router::build_for_test(state_with(m));
    let (_, j) = req_json(app, "GET", "/api/coude/999/players/111/steal-today", None).await;
    assert_eq!(j["count"], 3);
}
