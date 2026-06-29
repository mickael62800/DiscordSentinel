//! Tests d'integration HTTP pour les endpoints coude/inventory.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use chrono::TimeZone;
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;
use sentinel_core::domain::entities::coude::inventory::Insurance;
use sentinel_core::domain::entities::coude::inventory::InventoryItem;
use sentinel_core::domain::entities::coude::inventory::NewCoudePrime;
use sentinel_core::domain::entities::coude::inventory::Prime;
use sentinel_core::domain::errors::DomainError;

#[derive(Default)]
struct MockInventory {
    added: Mutex<Vec<(String, String, String)>>,
    used: Mutex<Vec<(String, String, String)>>,
    primes_created: Mutex<Vec<NewCoudePrime>>,
    claimed: Mutex<Vec<(String, String, String, String)>>,
    insurance_active: Mutex<bool>,
    insurance_bought: Mutex<bool>,
    expired: Mutex<Vec<Uuid>>,
    has_item_flag: Mutex<bool>,
    use_item_result: Mutex<bool>,
    buy_insurance_inserted: Mutex<bool>,
}

impl MockInventory {
    fn new() -> Self {
        let s = Self::default();
        *s.use_item_result.lock().unwrap() = true;
        *s.buy_insurance_inserted.lock().unwrap() = true;
        s
    }
}

#[async_trait]
impl ManageCoudeInventoryUseCase for MockInventory {
    async fn list_inventory(&self, g: &str, u: &str) -> Result<Vec<InventoryItem>, DomainError> {
        Ok(vec![InventoryItem {
            guild_id: g.into(),
            user_id: u.into(),
            item_key: "potion".into(),
            quantity: 3,
        }])
    }
    async fn add_item(&self, g: &str, u: &str, k: &str) -> Result<(), DomainError> {
        self.added
            .lock()
            .unwrap()
            .push((g.into(), u.into(), k.into()));
        Ok(())
    }
    async fn use_item(&self, g: &str, u: &str, k: &str) -> Result<bool, DomainError> {
        self.used
            .lock()
            .unwrap()
            .push((g.into(), u.into(), k.into()));
        Ok(*self.use_item_result.lock().unwrap())
    }
    async fn has_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(*self.has_item_flag.lock().unwrap())
    }
    async fn create_prime(&self, n: NewCoudePrime) -> Result<Prime, DomainError> {
        let p = Prime {
            id: Uuid::new_v4(),
            guild_id: n.guild_id.clone(),
            target_id: n.target_id.clone(),
            target_name: n.target_name.clone(),
            placed_by_id: n.placed_by_id.clone(),
            placed_by_name: n.placed_by_name.clone(),
            amount: n.amount,
            claimed: false,
            claimed_by_id: None,
            claimed_by_name: None,
            claimed_at: None,
            created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        };
        self.primes_created.lock().unwrap().push(n);
        Ok(p)
    }
    async fn list_active_primes(&self, _: &str, _: &str) -> Result<Vec<Prime>, DomainError> {
        Ok(vec![])
    }
    async fn claim_primes(&self, g: &str, t: &str, c: &str, n: &str) -> Result<i64, DomainError> {
        self.claimed
            .lock()
            .unwrap()
            .push((g.into(), t.into(), c.into(), n.into()));
        Ok(750)
    }
    async fn buy_insurance(&self, _: &str, _: &str, _: bool, _: i64) -> Result<bool, DomainError> {
        let r = *self.buy_insurance_inserted.lock().unwrap();
        *self.insurance_bought.lock().unwrap() = r;
        Ok(r)
    }
    async fn get_active_insurance(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<Insurance>, DomainError> {
        if *self.insurance_active.lock().unwrap() {
            Ok(Some(Insurance {
                id: Uuid::new_v4(),
                is_scam: true,
                expires_at: Utc.with_ymd_and_hms(2026, 12, 31, 23, 59, 59).unwrap(),
            }))
        } else {
            Ok(None)
        }
    }
    async fn expire_insurance(&self, id: Uuid) -> Result<(), DomainError> {
        self.expired.lock().unwrap().push(id);
        Ok(())
    }
}

fn state_with(m: Arc<MockInventory>) -> sentinel_api::adapters::inbound::http::state::AppState {
    let mut s = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    s.coude_inventory_uc = m;
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

// ── Items ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_inventory_returns_items() {
    let m = Arc::new(MockInventory::new());
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "GET", "/api/coude/999/inventory/111", None).await;
    assert_eq!(s, StatusCode::OK);
    let arr = j.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["item_key"], "potion");
    assert_eq!(arr[0]["quantity"], 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_item_forwards_key() {
    let m = Arc::new(MockInventory::new());
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({ "item_key": "sword" });
    let (s, _) = req_json(app, "POST", "/api/coude/999/inventory/111/add", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(
        m.added.lock().unwrap()[0],
        ("999".into(), "111".into(), "sword".into())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn use_item_returns_success_bool() {
    let m = Arc::new(MockInventory::new());
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({ "item_key": "potion" });
    let (s, j) = req_json(app, "POST", "/api/coude/999/inventory/111/use", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["success"], true);
    assert_eq!(m.used.lock().unwrap()[0].2, "potion");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn use_item_false_when_uc_returns_false() {
    let m = Arc::new(MockInventory::new());
    *m.use_item_result.lock().unwrap() = false;
    let app = router::build_for_test(state_with(m));
    let body = serde_json::json!({ "item_key": "potion" });
    let (_, j) = req_json(app, "POST", "/api/coude/999/inventory/111/use", Some(body)).await;
    assert_eq!(j["success"], false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn has_item_returns_bool() {
    let m = Arc::new(MockInventory::new());
    *m.has_item_flag.lock().unwrap() = true;
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "GET", "/api/coude/999/inventory/111/has/sword", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["has_item"], true);
}

// ── Primes ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_prime_returns_full_dto() {
    let m = Arc::new(MockInventory::new());
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({
        "target_id": "111", "target_name": "Tgt",
        "placed_by_id": "222", "placed_by_name": "Placer",
        "amount": 500
    });
    let (s, j) = req_json(app, "POST", "/api/coude/999/primes", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["target_id"], "111");
    assert_eq!(j["amount"], 500);
    assert_eq!(j["claimed"], false);
    assert_eq!(m.primes_created.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_active_primes_empty_array() {
    let m = Arc::new(MockInventory::new());
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "GET", "/api/coude/999/primes/111/active", None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(j.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claim_primes_returns_total() {
    let m = Arc::new(MockInventory::new());
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({
        "target_id": "111", "claimer_id": "222", "claimer_name": "Hunter"
    });
    let (s, j) = req_json(app, "POST", "/api/coude/999/primes/claim", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["total_claimed"], 750);
    let claimed = m.claimed.lock().unwrap();
    assert_eq!(
        claimed[0],
        ("999".into(), "111".into(), "222".into(), "Hunter".into())
    );
}

// ── Insurance ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn buy_insurance_success_204() {
    let m = Arc::new(MockInventory::new());
    let app = router::build_for_test(state_with(m));
    let body = serde_json::json!({
        "user_id": "111", "is_scam": true, "duration_seconds": 3600
    });
    let (s, _) = req_json(app, "POST", "/api/coude/999/insurance/buy", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn buy_insurance_already_active_returns_conflict() {
    let m = Arc::new(MockInventory::new());
    *m.buy_insurance_inserted.lock().unwrap() = false;
    let app = router::build_for_test(state_with(m));
    let body = serde_json::json!({
        "user_id": "111", "is_scam": false, "duration_seconds": 3600
    });
    let (s, j) = req_json(app, "POST", "/api/coude/999/insurance/buy", Some(body)).await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert!(j["error"].as_str().unwrap().contains("deja"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_active_insurance_none_returns_null() {
    let m = Arc::new(MockInventory::new());
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "GET", "/api/coude/999/insurance/111", None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(j.is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_active_insurance_some_returns_dto() {
    let m = Arc::new(MockInventory::new());
    *m.insurance_active.lock().unwrap() = true;
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "GET", "/api/coude/999/insurance/111", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["is_scam"], true);
    assert!(j["expires_at"].as_str().unwrap().starts_with("2026-12-31"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expire_insurance_invalid_uuid_422() {
    let m = Arc::new(MockInventory::new());
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "POST", "/api/coude/insurance/not-uuid/expire", None).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(j["error"].as_str().unwrap().contains("assurance"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expire_insurance_success_204() {
    let m = Arc::new(MockInventory::new());
    let app = router::build_for_test(state_with(m.clone()));
    let id = Uuid::new_v4();
    let (s, _) = req_json(
        app,
        "POST",
        &format!("/api/coude/insurance/{id}/expire"),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(m.expired.lock().unwrap()[0], id);
}
