//! Tests d'integration HTTP pour les endpoints coude/taunts.

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

use sentinel_core::domain::enums::system::role::Role;
use sentinel_api::adapters::inbound::http::router;
use sentinel_core::domain::entities::coude::taunt::TauntsConfig;
use sentinel_core::domain::entities::coude::taunt::TauntEvent;
use sentinel_core::domain::errors::DomainError;
use sentinel_api::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;

struct MockTaunts {
    channel_sets: Mutex<Vec<(String, Option<String>)>>,
    enabled_sets: Mutex<Vec<(String, bool)>>,
    rename_sets: Mutex<Vec<(String, bool)>>,
    messages_sets: Mutex<Vec<(String, bool)>>,
    opt_out_calls: Mutex<Vec<(String, String, bool)>>,
    emit_on_trigger: Mutex<bool>,
    last_jackpot_amount: Mutex<i64>,
    opt_outs_list: Mutex<Vec<String>>,
    config: Mutex<TauntsConfig>,
}

impl MockTaunts {
    fn new() -> Self {
        Self {
            channel_sets: Mutex::new(vec![]),
            enabled_sets: Mutex::new(vec![]),
            rename_sets: Mutex::new(vec![]),
            messages_sets: Mutex::new(vec![]),
            opt_out_calls: Mutex::new(vec![]),
            emit_on_trigger: Mutex::new(false),
            last_jackpot_amount: Mutex::new(0),
            opt_outs_list: Mutex::new(vec![]),
            config: Mutex::new(TauntsConfig {
                guild_id: "999".into(),
                channel_id: Some("chan-1".into()),
                enabled: true,
                rename_enabled: true,
                messages_enabled: true,
            }),
        }
    }
    fn sample_event() -> TauntEvent {
        TauntEvent {
            channel_id: "chan-1".into(),
            target_user_id: "u1".into(),
            message: "test".into(),
            nickname_suffix: "[rekt]".into(),
            streak_kind: "bj_win",
            streak_value: 3,
        }
    }
}

#[async_trait]
impl ManageCoudeTauntsUseCase for MockTaunts {
    async fn on_player_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_lost(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_drew(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn on_player_stolen_from(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_defended_steal(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }

    async fn on_bj_natural(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(if *self.emit_on_trigger.lock().unwrap() { Some(Self::sample_event()) } else { None })
    }
    async fn on_bj_hand_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(if *self.emit_on_trigger.lock().unwrap() { Some(Self::sample_event()) } else { None })
    }
    async fn on_bj_hand_bust(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(if *self.emit_on_trigger.lock().unwrap() { Some(Self::sample_event()) } else { None })
    }
    async fn on_bankruptcy(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(if *self.emit_on_trigger.lock().unwrap() { Some(Self::sample_event()) } else { None })
    }
    async fn on_jackpot(&self, _: &str, _: &str, amount: i64) -> Result<Option<TauntEvent>, DomainError> {
        *self.last_jackpot_amount.lock().unwrap() = amount;
        Ok(if *self.emit_on_trigger.lock().unwrap() { Some(Self::sample_event()) } else { None })
    }
    async fn on_generous_donor(&self, _: &str, _: &str, amount: i64) -> Result<Option<TauntEvent>, DomainError> {
        *self.last_jackpot_amount.lock().unwrap() = amount;
        Ok(if *self.emit_on_trigger.lock().unwrap() { Some(Self::sample_event()) } else { None })
    }

    async fn get_config(&self, _: &str) -> Result<TauntsConfig, DomainError> {
        Ok(self.config.lock().unwrap().clone())
    }
    async fn set_channel(&self, g: &str, c: Option<&str>) -> Result<(), DomainError> {
        self.channel_sets.lock().unwrap().push((g.into(), c.map(String::from)));
        Ok(())
    }
    async fn set_enabled(&self, g: &str, e: bool) -> Result<(), DomainError> {
        self.enabled_sets.lock().unwrap().push((g.into(), e));
        Ok(())
    }
    async fn set_rename_enabled(&self, g: &str, e: bool) -> Result<(), DomainError> {
        self.rename_sets.lock().unwrap().push((g.into(), e));
        Ok(())
    }
    async fn set_messages_enabled(&self, g: &str, e: bool) -> Result<(), DomainError> {
        self.messages_sets.lock().unwrap().push((g.into(), e));
        Ok(())
    }
    async fn set_opt_out(&self, g: &str, u: &str, o: bool) -> Result<(), DomainError> {
        self.opt_out_calls.lock().unwrap().push((g.into(), u.into(), o));
        Ok(())
    }
    async fn is_opted_out(&self, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn list_opt_outs(&self, _: &str) -> Result<Vec<String>, DomainError> {
        Ok(self.opt_outs_list.lock().unwrap().clone())
    }
}

fn state_with(m: Arc<MockTaunts>) -> sentinel_api::adapters::inbound::http::state::AppState {
    let mut s = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    s.coude_taunts_uc = m;
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

// ── Config GET ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_taunts_config_returns_merged_opt_outs() {
    let m = Arc::new(MockTaunts::new());
    *m.opt_outs_list.lock().unwrap() = vec!["user-a".into(), "user-b".into()];
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "GET", "/api/coude/999/config/taunts", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["guild_id"], "999");
    assert_eq!(j["channel_id"], "chan-1");
    assert_eq!(j["enabled"], true);
    assert_eq!(j["rename_enabled"], true);
    assert_eq!(j["messages_enabled"], true);
    let outs = j["opt_outs"].as_array().unwrap();
    assert_eq!(outs.len(), 2);
}

// ── Config PUT + RBAC ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_taunts_config_without_rbac_passes_through() {
    let m = Arc::new(MockTaunts::new());
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({
        "channel_id": "chan-2", "enabled": false,
        "rename_enabled": true, "messages_enabled": false
    });
    let (s, _) = req_json(app, "PUT", "/api/coude/999/config/taunts", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(m.channel_sets.lock().unwrap()[0].1.as_deref(), Some("chan-2"));
    assert_eq!(m.enabled_sets.lock().unwrap()[0].1, false);
    assert_eq!(m.rename_sets.lock().unwrap()[0].1, true);
    assert_eq!(m.messages_sets.lock().unwrap()[0].1, false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_taunts_config_moderator_forbidden() {
    let m = Arc::new(MockTaunts::new());
    let app = router::build_for_test(state_with(m));
    let req = test_helpers::request_with_rbac(
        "PUT", "/api/coude/999/config/taunts",
        "u1", Some(Role::Moderator), Some("999".into()),
        Some(serde_json::json!({"channel_id": null, "enabled": true})),
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_taunts_config_admin_ok() {
    let m = Arc::new(MockTaunts::new());
    let app = router::build_for_test(state_with(m.clone()));
    let req = test_helpers::request_with_rbac(
        "PUT", "/api/coude/999/config/taunts",
        "u1", Some(Role::Admin), Some("999".into()),
        Some(serde_json::json!({"channel_id": null, "enabled": true})),
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_taunts_config_skips_optional_fields_when_absent() {
    let m = Arc::new(MockTaunts::new());
    let app = router::build_for_test(state_with(m.clone()));
    // Pas de rename_enabled / messages_enabled : les setters correspondants
    // ne doivent pas etre appeles.
    let body = serde_json::json!({ "channel_id": "chan-9", "enabled": true });
    let (s, _) = req_json(app, "PUT", "/api/coude/999/config/taunts", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert!(m.rename_sets.lock().unwrap().is_empty());
    assert!(m.messages_sets.lock().unwrap().is_empty());
    assert_eq!(m.channel_sets.lock().unwrap().len(), 1);
    assert_eq!(m.enabled_sets.lock().unwrap().len(), 1);
}

// ── Tracking ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn track_bj_natural_none_returns_event_null() {
    let m = Arc::new(MockTaunts::new());
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "POST", "/api/coude/999/taunts/bj/natural/111", None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["event"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn track_bj_won_emits_event_when_threshold_crossed() {
    let m = Arc::new(MockTaunts::new());
    *m.emit_on_trigger.lock().unwrap() = true;
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "POST", "/api/coude/999/taunts/bj/won/111", None).await;
    assert_eq!(s, StatusCode::OK);
    let ev = &j["event"];
    assert_eq!(ev["channel_id"], "chan-1");
    assert_eq!(ev["target_user_id"], "u1");
    assert_eq!(ev["streak_kind"], "bj_win");
    assert_eq!(ev["streak_value"], 3);
    assert_eq!(ev["nickname_suffix"], "[rekt]");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn track_bj_bust_routes_to_bust_handler() {
    let m = Arc::new(MockTaunts::new());
    *m.emit_on_trigger.lock().unwrap() = true;
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "POST", "/api/coude/999/taunts/bj/bust/111", None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(!j["event"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn track_bankruptcy_returns_optional_event() {
    let m = Arc::new(MockTaunts::new());
    let app = router::build_for_test(state_with(m));
    let (s, j) = req_json(app, "POST", "/api/coude/999/taunts/eco/bankruptcy/111", None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["event"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn track_jackpot_forwards_amount() {
    let m = Arc::new(MockTaunts::new());
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({ "amount": 15000 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/taunts/eco/jackpot/111", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(*m.last_jackpot_amount.lock().unwrap(), 15000);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn track_generous_donor_forwards_amount() {
    let m = Arc::new(MockTaunts::new());
    let app = router::build_for_test(state_with(m.clone()));
    let body = serde_json::json!({ "amount": 2000 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/taunts/eco/donor/111", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(*m.last_jackpot_amount.lock().unwrap(), 2000);
}

// ── Opt-out removal ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remove_opt_out_moderator_forbidden() {
    let m = Arc::new(MockTaunts::new());
    let app = router::build_for_test(state_with(m));
    let req = test_helpers::request_with_rbac(
        "DELETE", "/api/coude/999/config/taunts/opt-outs/111",
        "u1", Some(Role::Moderator), Some("999".into()), None,
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remove_opt_out_admin_calls_set_opt_out_false() {
    let m = Arc::new(MockTaunts::new());
    let app = router::build_for_test(state_with(m.clone()));
    let req = test_helpers::request_with_rbac(
        "DELETE", "/api/coude/999/config/taunts/opt-outs/111",
        "u1", Some(Role::Admin), Some("999".into()), None,
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let calls = m.opt_out_calls.lock().unwrap();
    assert_eq!(calls[0], ("999".into(), "111".into(), false));
}
