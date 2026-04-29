//! Tests d'integration HTTP pour les endpoints coude/players.

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

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::domain::entities::coude::player::CombatStat;
use sentinel_api::domain::entities::coude::player::Player;
use sentinel_api::domain::entities::coude::taunt::TauntEvent;
use sentinel_api::domain::entities::coude::player::XpProgress;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use sentinel_api::ports::inbound::casino::manage_wallet::TxWalletMutation;
use sentinel_api::ports::inbound::casino::manage_wallet::WalletMutation;
use sentinel_api::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase;

fn sample_player(guild: &str, user: &str) -> Player {
    Player {
        guild_id: guild.into(), user_id: user.into(), username: "Alice".into(),
        coins: 500,
        total_wins: 2, total_losses: 1, total_draws: 0,
        total_earned: 200, total_lost: 50, total_stolen: 10,
        cowardice_count: 0, chaos_events: 0,
        casino_wins: 0, casino_losses: 0,
        level: 3, xp: 120, stat_points: 1,
        atk: 5, def: 5,
        class: None, title: None, class_changed_at: None,
        hp_current: 80, hp_max: 100,
        hp_last_regen: None, repos_last_used: None,
        season: 1,
        created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0).unwrap(),
    }
}

#[derive(Default)]
struct MockPlayers {
    created: Mutex<Vec<(String, String, String)>>,
    xp_added: Mutex<Vec<(String, String, i64)>>,
    class_updates: Mutex<Vec<(String, String, String)>>,
    stat_spends: Mutex<Vec<(String, String, CombatStat)>>,
    reset_stats: Mutex<Vec<(String, String, i64)>>,
    wins: Mutex<Vec<(String, String, i64, i64)>>,
    losses: Mutex<Vec<(String, String, i64)>>,
    draws: Mutex<Vec<(String, String, i64)>>,
    cowardice: Mutex<Vec<(String, String)>>,
    chaos: Mutex<Vec<(String, String)>>,
    earned: Mutex<Vec<(String, String, i64)>>,
    lost: Mutex<Vec<(String, String, i64)>>,
    hp_updates: Mutex<Vec<(String, String, i32, i32)>>,
    full_heals: Mutex<Vec<(String, String)>>,
    fail_get: Mutex<bool>,
    random_count_received: Mutex<i64>,
    cowardice_return: Mutex<i32>,
    guild_ids: Mutex<Vec<String>>,
}

#[async_trait]
impl ManageCoudePlayersUseCase for MockPlayers {
    async fn get_or_create(&self, g: String, u: String, n: String) -> Result<Player, DomainError> {
        self.created.lock().unwrap().push((g.clone(), u.clone(), n));
        Ok(sample_player(&g, &u))
    }
    async fn get(&self, g: &str, u: &str) -> Result<Player, DomainError> {
        if *self.fail_get.lock().unwrap() {
            return Err(DomainError::NotFound("joueur".into()));
        }
        Ok(sample_player(g, u))
    }
    async fn list(&self, g: &str) -> Result<Vec<Player>, DomainError> {
        Ok(vec![sample_player(g, "u1")])
    }
    async fn random_active(&self, g: &str, count: i64) -> Result<Vec<Player>, DomainError> {
        *self.random_count_received.lock().unwrap() = count;
        Ok((0..count).map(|i| sample_player(g, &format!("u{i}"))).collect())
    }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> {
        Ok(self.guild_ids.lock().unwrap().clone())
    }
    async fn update_class(&self, g: &str, u: &str, c: &str) -> Result<(), DomainError> {
        self.class_updates.lock().unwrap().push((g.into(), u.into(), c.into()));
        Ok(())
    }
    async fn add_xp(&self, g: &str, u: &str, amt: i64) -> Result<XpProgress, DomainError> {
        self.xp_added.lock().unwrap().push((g.into(), u.into(), amt));
        Ok(XpProgress { new_xp: 170, new_level: 4, leveled_up: true, stat_points_gained: 1 })
    }
    async fn spend_stat_point(&self, g: &str, u: &str, s: CombatStat) -> Result<Player, DomainError> {
        self.stat_spends.lock().unwrap().push((g.into(), u.into(), s));
        Ok(sample_player(g, u))
    }
    async fn reset_stats(&self, g: &str, u: &str, cost: i64) -> Result<Player, DomainError> {
        self.reset_stats.lock().unwrap().push((g.into(), u.into(), cost));
        Ok(sample_player(g, u))
    }
    async fn record_win(&self, g: &str, u: &str, e: i64, s: i64) -> Result<(), DomainError> {
        self.wins.lock().unwrap().push((g.into(), u.into(), e, s));
        Ok(())
    }
    async fn record_loss(&self, g: &str, u: &str, l: i64) -> Result<(), DomainError> {
        self.losses.lock().unwrap().push((g.into(), u.into(), l));
        Ok(())
    }
    async fn record_draw(&self, g: &str, u: &str, l: i64) -> Result<(), DomainError> {
        self.draws.lock().unwrap().push((g.into(), u.into(), l));
        Ok(())
    }
    async fn increment_cowardice(&self, g: &str, u: &str) -> Result<i32, DomainError> {
        self.cowardice.lock().unwrap().push((g.into(), u.into()));
        Ok(*self.cowardice_return.lock().unwrap())
    }
    async fn increment_chaos(&self, g: &str, u: &str) -> Result<(), DomainError> {
        self.chaos.lock().unwrap().push((g.into(), u.into()));
        Ok(())
    }
    async fn record_coins_earned(&self, g: &str, u: &str, a: i64) -> Result<(), DomainError> {
        self.earned.lock().unwrap().push((g.into(), u.into(), a));
        Ok(())
    }
    async fn record_coins_lost(&self, g: &str, u: &str, a: i64) -> Result<(), DomainError> {
        self.lost.lock().unwrap().push((g.into(), u.into(), a));
        Ok(())
    }
    async fn update_hp(&self, g: &str, u: &str, c: i32, m: i32) -> Result<(), DomainError> {
        self.hp_updates.lock().unwrap().push((g.into(), u.into(), c, m));
        Ok(())
    }
    async fn full_heal(&self, g: &str, u: &str) -> Result<(), DomainError> {
        self.full_heals.lock().unwrap().push((g.into(), u.into()));
        Ok(())
    }
    async fn regen_hp_tick(&self, _: f64, _: f64, _: f64, _: f64) -> Result<u64, DomainError> {
        Ok(0)
    }
}

#[derive(Default)]
struct MockWallet {
    credits: Mutex<Vec<(String, String, i64, String)>>,
    debits: Mutex<Vec<(String, String, i64, String)>>,
    balance_to_return: Mutex<i64>,
}

#[async_trait]
impl ManageWalletUseCase for MockWallet {
    async fn credit(&self, g: &str, u: &str, a: i64, src: &str, _: &str) -> Result<WalletMutation, DomainError> {
        self.credits.lock().unwrap().push((g.into(), u.into(), a, src.into()));
        Ok(WalletMutation { new_balance: a, previous_balance: 0, triggered_taunts: vec![] })
    }
    async fn debit(&self, g: &str, u: &str, a: i64, src: &str, _: &str) -> Result<WalletMutation, DomainError> {
        self.debits.lock().unwrap().push((g.into(), u.into(), a, src.into()));
        Ok(WalletMutation { new_balance: 0, previous_balance: a, triggered_taunts: vec![] })
    }
    async fn transfer(&self, _: &str, _: &str, _: &str, _: i64, _: &str, _: &str)
        -> Result<Vec<TauntEvent>, DomainError> { Ok(vec![]) }
    async fn get_balance(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(*self.balance_to_return.lock().unwrap())
    }
    async fn credit_tx(&self, _: &mut sqlx::Transaction<'_, sqlx::Postgres>, _: &str, _: &str, _: i64, _: &str, _: &str)
        -> Result<TxWalletMutation, DomainError> { unimplemented!() }
    async fn debit_tx(&self, _: &mut sqlx::Transaction<'_, sqlx::Postgres>, _: &str, _: &str, _: i64, _: &str, _: &str)
        -> Result<TxWalletMutation, DomainError> { unimplemented!() }
    async fn post_commit_taunts(&self, _: &str, _: &str, _: &TxWalletMutation) -> Vec<TauntEvent> { vec![] }
}

fn state_with(p: Arc<MockPlayers>, w: Arc<MockWallet>) -> sentinel_api::adapters::inbound::http::state::AppState {
    let mut s = test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels));
    s.coude_players_uc = p;
    s.wallet_uc = w;
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

// ── Listing ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_players_returns_dtos() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p, Arc::new(MockWallet::default())));
    let (s, j) = req_json(app, "GET", "/api/coude/999/players", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j.as_array().unwrap().len(), 1);
    assert_eq!(j[0]["username"], "Alice");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_all_guild_ids_returns_list() {
    let p = Arc::new(MockPlayers::default());
    *p.guild_ids.lock().unwrap() = vec!["g1".into(), "g2".into()];
    let app = router::build_for_test(state_with(p, Arc::new(MockWallet::default())));
    let (s, j) = req_json(app, "GET", "/api/coude/guilds", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j.as_array().unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn random_players_default_count_is_2() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let (s, j) = req_json(app, "GET", "/api/coude/999/players/random", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j.as_array().unwrap().len(), 2);
    assert_eq!(*p.random_count_received.lock().unwrap(), 2); // DEFAULT_COUDE_OPPONENT_COUNT
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn random_players_custom_count_forwarded() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let (_, _) = req_json(app, "GET", "/api/coude/999/players/random?count=5", None).await;
    assert_eq!(*p.random_count_received.lock().unwrap(), 5);
}

// ── CRUD ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_or_create_player_forwards_payload() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "user_id": "111", "username": "Bob" });
    let (s, j) = req_json(app, "POST", "/api/coude/999/players/get-or-create", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["user_id"], "111");
    assert_eq!(p.created.lock().unwrap()[0], ("999".into(), "111".into(), "Bob".into()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_player_found_returns_full_dto() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p, Arc::new(MockWallet::default())));
    let (s, j) = req_json(app, "GET", "/api/coude/999/players/111", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["coins"], 500);
    assert_eq!(j["level"], 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_player_not_found_returns_404() {
    let p = Arc::new(MockPlayers::default());
    *p.fail_get.lock().unwrap() = true;
    let app = router::build_for_test(state_with(p, Arc::new(MockWallet::default())));
    let (s, _) = req_json(app, "GET", "/api/coude/999/players/111", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_player_class_forwards() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "class": "warrior" });
    let (s, _) = req_json(app, "PATCH", "/api/coude/999/players/111/class", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(p.class_updates.lock().unwrap()[0].2, "warrior");
}

// ── Progression ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_xp_returns_progress() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "amount": 50 });
    let (s, j) = req_json(app, "POST", "/api/coude/999/players/111/xp", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["new_level"], 4);
    assert_eq!(j["leveled_up"], true);
    assert_eq!(p.xp_added.lock().unwrap()[0].2, 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spend_stat_atk_accepted() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "stat": "atk" });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/spend-stat", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(p.stat_spends.lock().unwrap()[0].2, CombatStat::Atk);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spend_stat_def_accepted() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "stat": "def" });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/spend-stat", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(p.stat_spends.lock().unwrap()[0].2, CombatStat::Def);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spend_stat_invalid_returns_422() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p, Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "stat": "luck" });
    let (s, j) = req_json(app, "POST", "/api/coude/999/players/111/spend-stat", Some(body)).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(j["error"].as_str().unwrap().contains("atk"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_stats_forwards_cost() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "cost": 300 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/reset-stats", Some(body)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(p.reset_stats.lock().unwrap()[0].2, 300);
}

// ── Stats recording ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_win_forwards_earned_and_stolen() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "earned": 100, "stolen": 50 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/record-win", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(p.wins.lock().unwrap()[0], ("999".into(), "111".into(), 100, 50));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_loss_forwards() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "lost": 25 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/record-loss", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(p.losses.lock().unwrap()[0].2, 25);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_draw_forwards() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "lost": 10 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/record-draw", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(p.draws.lock().unwrap()[0].2, 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn increment_cowardice_returns_new_count() {
    let p = Arc::new(MockPlayers::default());
    *p.cowardice_return.lock().unwrap() = 4;
    let app = router::build_for_test(state_with(p, Arc::new(MockWallet::default())));
    let (s, j) = req_json(app, "POST", "/api/coude/999/players/111/increment-cowardice", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["cowardice_count"], 4);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn increment_chaos_204() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/increment-chaos", None).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(p.chaos.lock().unwrap().len(), 1);
}

// ── Coins (via wallet UC) ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adjust_coins_zero_delta_short_circuits() {
    let p = Arc::new(MockPlayers::default());
    let w = Arc::new(MockWallet::default());
    let app = router::build_for_test(state_with(p, w.clone()));
    let body = serde_json::json!({ "amount": 0 });
    let (s, _) = req_json(app, "PATCH", "/api/coude/players/999/111/coins", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert!(w.credits.lock().unwrap().is_empty());
    assert!(w.debits.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adjust_coins_positive_credits_wallet() {
    let w = Arc::new(MockWallet::default());
    let app = router::build_for_test(state_with(Arc::new(MockPlayers::default()), w.clone()));
    let body = serde_json::json!({ "amount": 100 });
    let (s, _) = req_json(app, "PATCH", "/api/coude/players/999/111/coins", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let credits = w.credits.lock().unwrap();
    assert_eq!(credits[0], ("999".into(), "111".into(), 100, "coude_adjust".into()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adjust_coins_negative_debits_wallet_with_abs_value() {
    let w = Arc::new(MockWallet::default());
    let app = router::build_for_test(state_with(Arc::new(MockPlayers::default()), w.clone()));
    let body = serde_json::json!({ "amount": -75 });
    let (_, _) = req_json(app, "PATCH", "/api/coude/players/999/111/coins", Some(body)).await;
    assert_eq!(w.debits.lock().unwrap()[0].2, 75);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coins_earned_rejects_zero_or_negative() {
    let app = router::build_for_test(state_with(Arc::new(MockPlayers::default()), Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "amount": 0 });
    let (s, j) = req_json(app, "POST", "/api/coude/999/players/111/coins-earned", Some(body)).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(j["error"].as_str().unwrap().contains("positif"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coins_earned_credits_wallet_and_updates_stats() {
    let p = Arc::new(MockPlayers::default());
    let w = Arc::new(MockWallet::default());
    let app = router::build_for_test(state_with(p.clone(), w.clone()));
    let body = serde_json::json!({ "amount": 200 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/coins-earned", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(w.credits.lock().unwrap()[0].3, "coude_earn");
    assert_eq!(p.earned.lock().unwrap()[0].2, 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coins_lost_rejects_negative() {
    let app = router::build_for_test(state_with(Arc::new(MockPlayers::default()), Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "amount": -5 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/coins-lost", Some(body)).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coins_lost_clamps_to_balance_when_amount_exceeds() {
    let p = Arc::new(MockPlayers::default());
    let w = Arc::new(MockWallet::default());
    *w.balance_to_return.lock().unwrap() = 30;
    let app = router::build_for_test(state_with(p.clone(), w.clone()));
    let body = serde_json::json!({ "amount": 100 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/coins-lost", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    // clamp_debit_to_balance(100, 30) = 30
    assert_eq!(w.debits.lock().unwrap()[0].2, 30);
    assert_eq!(p.lost.lock().unwrap()[0].2, 30);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coins_lost_zero_balance_skips_debit_but_records_stat_zero() {
    let p = Arc::new(MockPlayers::default());
    let w = Arc::new(MockWallet::default());
    *w.balance_to_return.lock().unwrap() = 0;
    let app = router::build_for_test(state_with(p.clone(), w.clone()));
    let body = serde_json::json!({ "amount": 50 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/coins-lost", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert!(w.debits.lock().unwrap().is_empty());
    assert_eq!(p.lost.lock().unwrap()[0].2, 0);
}

// ── HP ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_hp_forwards_current_and_max() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let body = serde_json::json!({ "hp_current": 50, "hp_max": 100 });
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/hp", Some(body)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(p.hp_updates.lock().unwrap()[0], ("999".into(), "111".into(), 50, 100));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repos_triggers_full_heal() {
    let p = Arc::new(MockPlayers::default());
    let app = router::build_for_test(state_with(p.clone(), Arc::new(MockWallet::default())));
    let (s, _) = req_json(app, "POST", "/api/coude/999/players/111/repos", None).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(p.full_heals.lock().unwrap()[0], ("999".into(), "111".into()));
}
