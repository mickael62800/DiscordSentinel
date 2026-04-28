//! Tests d'integration HTTP pour les endpoints coude/combats.

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
use sentinel_api::domain::entities::coude::combat::CombatResolution;
use sentinel_api::domain::entities::coude::combat::CoudeCombat;
use sentinel_api::domain::entities::coude::combat::NewCoudeCombat;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::coude::manage_combats::ManageCoudeCombatsUseCase;

fn sample_combat(id: Uuid, guild: &str, status: &str) -> CoudeCombat {
    CoudeCombat {
        id, guild_id: guild.into(),
        channel_id: Some("chan".into()),
        attacker_id: "a1".into(), attacker_name: "Att".into(),
        defender_id: "d1".into(), defender_name: "Def".into(),
        mise: 100, status: status.into(),
        winner_id: None,
        attacker_roll: None, defender_roll: None,
        chaos_event: None, special_attack: None, defender_special: None,
        coins_transferred: None, result_message: None, message_id: None,
        created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        accepted_at: None, resolved_at: None,
    }
}

#[derive(Default)]
struct MockCombats {
    list_calls: Mutex<Vec<(String, Option<String>, i64)>>,
    created: Mutex<Vec<NewCoudeCombat>>,
    resolved: Mutex<Vec<(Uuid, CombatResolution)>>,
    cancelled: Mutex<Vec<Uuid>>,
    betting_set: Mutex<Vec<(Uuid, String)>>,
    expired: Mutex<Vec<Uuid>>,
    defender_specials: Mutex<Vec<(Uuid, String)>>,
    canned_get: Mutex<Option<CoudeCombat>>,
    set_betting_result: Mutex<bool>,
}

#[async_trait]
impl ManageCoudeCombatsUseCase for MockCombats {
    async fn list(&self, g: &str, status: Option<&str>, limit: i64) -> Result<Vec<CoudeCombat>, DomainError> {
        self.list_calls.lock().unwrap().push((g.into(), status.map(String::from), limit));
        Ok(vec![sample_combat(Uuid::new_v4(), g, "pending")])
    }
    async fn get(&self, id: Uuid) -> Result<CoudeCombat, DomainError> {
        Ok(self.canned_get.lock().unwrap().clone()
            .unwrap_or_else(|| sample_combat(id, "999", "resolved")))
    }
    async fn get_pending_for_attacker(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> {
        Ok(None)
    }
    async fn get_pending_for_defender(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> {
        Ok(Some(sample_combat(Uuid::new_v4(), "999", "pending")))
    }
    async fn list_expired_pending(&self) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
    async fn get_betting_for_participant(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> {
        Ok(None)
    }
    async fn create(&self, n: NewCoudeCombat) -> Result<CoudeCombat, DomainError> {
        let c = sample_combat(Uuid::new_v4(), &n.guild_id, "pending");
        self.created.lock().unwrap().push(n);
        Ok(c)
    }
    async fn cancel(&self, id: Uuid) -> Result<(), DomainError> {
        self.cancelled.lock().unwrap().push(id);
        Ok(())
    }
    async fn resolve(&self, id: Uuid, r: CombatResolution) -> Result<(), DomainError> {
        self.resolved.lock().unwrap().push((id, r));
        Ok(())
    }
    async fn set_betting(&self, id: Uuid, msg: &str) -> Result<bool, DomainError> {
        self.betting_set.lock().unwrap().push((id, msg.into()));
        Ok(*self.set_betting_result.lock().unwrap())
    }
    async fn expire(&self, id: Uuid) -> Result<(), DomainError> {
        self.expired.lock().unwrap().push(id);
        Ok(())
    }
    async fn set_defender_special(&self, id: Uuid, k: &str) -> Result<(), DomainError> {
        self.defender_specials.lock().unwrap().push((id, k.into()));
        Ok(())
    }
}

impl MockCombats {
    fn new() -> Self {
        let s = Self::default();
        *s.set_betting_result.lock().unwrap() = true;
        s
    }
}

fn state_with(m: Arc<MockCombats>) -> sentinel_api::adapters::inbound::http::state::AppState {
    let mut s = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    s.coude_combats_uc = m;
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

// ── Lecture ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_combats_default_limit_is_50() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m.clone()));
    let (s, j) = req_json(app, "GET", "/api/coude/999/combats", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j.as_array().unwrap().len(), 1);
    let calls = m.list_calls.lock().unwrap();
    assert_eq!(calls[0].2, 50); // DEFAULT_COUDE_COMBATS_LIMIT
    assert_eq!(calls[0].1, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_combats_with_status_and_limit_filters() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m.clone()));
    let (s, _) = req_json(app, "GET", "/api/coude/999/combats?status=pending&limit=5", None).await;
    assert_eq!(s, StatusCode::OK);
    let calls = m.list_calls.lock().unwrap();
    assert_eq!(calls[0].1.as_deref(), Some("pending"));
    assert_eq!(calls[0].2, 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_combat_invalid_uuid_422() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m));
    let (s, _) = req_json(app, "GET", "/api/coude/combats/bad/detail", None).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_combat_valid_uuid_returns_dto() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m));
    let id = Uuid::new_v4();
    let (s, j) = req_json(app, "GET", &format!("/api/coude/combats/{id}/detail"), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["id"], id.to_string());
    assert_eq!(j["status"], "resolved");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_pending_attacker_none_returns_null() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "GET", "/api/coude/999/combats/pending/attacker/111", None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(j.is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_pending_defender_some_returns_full_dto() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "GET", "/api/coude/999/combats/pending/defender/111", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["status"], "pending");
    assert_eq!(j["attacker_name"], "Att");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_expired_combats_returns_empty() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "GET", "/api/coude/combats/expired", None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(j.as_array().unwrap().is_empty());
}

// ── Cycle de vie ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_combat_forwards_payload() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({
        "channel_id": "ch1",
        "attacker_id": "a", "attacker_name": "A",
        "defender_id": "d", "defender_name": "D",
        "mise": 200, "special_attack": "lightning"
    });
    let (s, j) = req_json(app, "POST", "/api/coude/999/combats/create", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["guild_id"], "999");
    let created = m.created.lock().unwrap();
    assert_eq!(created[0].mise, 200);
    assert_eq!(created[0].special_attack.as_deref(), Some("lightning"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_combat_without_rbac_passes() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m.clone()));
    let id = Uuid::new_v4();
    let (s, j) = req_json(app, "DELETE", &format!("/api/coude/combats/{id}"), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(m.cancelled.lock().unwrap()[0], id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_combat_invalid_uuid_422() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m));
    let (s, _) = req_json(app, "DELETE", "/api/coude/combats/bad", None).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_combat_defaults_coins_transferred_to_zero() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m.clone()));
    let id = Uuid::new_v4();
    let body = serde_json::json!({
        "status": "resolved",
        "winner_id": "a",
        "attacker_roll": 10,
        "defender_roll": 5,
        "chaos_event": null,
        "result_message": null
    });
    let (s, _) = req_json(app, "POST", &format!("/api/coude/combats/{id}/resolve"), Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let resolved = m.resolved.lock().unwrap();
    assert_eq!(resolved[0].0, id);
    assert_eq!(resolved[0].1.coins_transferred, 0);
    assert_eq!(resolved[0].1.winner_id.as_deref(), Some("a"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_combat_propagates_coins_when_present() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m.clone()));
    let id = Uuid::new_v4();
    let body = serde_json::json!({
        "status": "resolved", "winner_id": "a",
        "attacker_roll": 10, "defender_roll": 5,
        "chaos_event": null, "result_message": null,
        "coins_transferred": 500
    });
    let (_, _) = req_json(app, "POST", &format!("/api/coude/combats/{id}/resolve"), Some(body)).await;
    assert_eq!(m.resolved.lock().unwrap()[0].1.coins_transferred, 500);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_combat_betting_returns_success_flag() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m.clone()));
    let id = Uuid::new_v4();
    let body = serde_json::json!({ "message_id": "msg-123" });
    let (s, j) = req_json(app, "POST", &format!("/api/coude/combats/{id}/betting"), Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["success"], true);
    assert_eq!(m.betting_set.lock().unwrap()[0], (id, "msg-123".into()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expire_combat_success() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m.clone()));
    let id = Uuid::new_v4();
    let (s, _) = req_json(app, "POST", &format!("/api/coude/combats/{id}/expire"), None).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(m.expired.lock().unwrap()[0], id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_defender_special_forwards_key() {
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m.clone()));
    let id = Uuid::new_v4();
    let body = serde_json::json!({ "item_key": "shield" });
    let (s, _) = req_json(app, "POST", &format!("/api/coude/combats/{id}/defender-special"), Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(m.defender_specials.lock().unwrap()[0], (id, "shield".into()));
}

// ── Purge ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_all_without_rbac_returns_7_table_totals() {
    // Sans rbac -> check_role_for_guild laisse passer (bot/internal).
    let m = Arc::new(MockCombats::new());
    // Guild isole (VARCHAR(20)) pour eviter d'impacter d'autres tests.
    let guild_id = format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128);
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "DELETE", &format!("/api/coude/{guild_id}/purge"), None).await;
    assert_eq!(s, StatusCode::OK);
    let obj = j.as_object().unwrap();
    // Les 7 tables du COUDE_PURGE_TABLES doivent etre presentes.
    assert_eq!(obj.len(), 7);
    for t in ["coude_insurances", "coude_bets", "coude_combats", "coude_primes",
              "coude_inventory", "coude_events", "coude_players"] {
        assert!(obj.contains_key(t), "table manquante: {t}");
        assert_eq!(obj[t], 0); // guild fraiche -> 0 rows affected
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_all_viewer_forbidden() {
    use sentinel_api::domain::enums::system::role::Role;
    let m = Arc::new(MockCombats::new());
    let app = router::build_for_test(state_with(m));
    let req = test_helpers::request_with_rbac(
        "DELETE", "/api/coude/999/purge",
        "u1", Some(Role::Viewer), Some("999".into()), None,
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
