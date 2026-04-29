use super::*;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::domain::entities::coude::player::CombatStat;
use crate::domain::entities::coude::player::Player;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::errors::DomainError;
use crate::domain::enums::coude::coude_class::PlayerClass;
use crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::casino::manage_wallet::TxWalletMutation;
use crate::ports::inbound::casino::manage_wallet::WalletMutation;
use sqlx::Postgres;
use sqlx::Transaction;
// ── Mocks ──

#[derive(Default)]
struct MockPlayersUc {
    player: Mutex<Option<Player>>,
    class_calls: Mutex<Vec<(String, String, String)>>,
    add_xp_calls: Mutex<Vec<(String, String, i64)>>,
    update_hp_calls: Mutex<Vec<(String, String, i32, i32)>>,
    regen_return: Mutex<u64>,
}

fn make_player() -> Player {
    let now = Utc::now();
    Player {
        guild_id: "g".into(), user_id: "u".into(), username: "Alice".into(),
        coins: 100,
        total_wins: 0, total_losses: 0, total_draws: 0,
        total_earned: 0, total_lost: 0, total_stolen: 0,
        cowardice_count: 0, chaos_events: 0, casino_wins: 0, casino_losses: 0,
        level: 1, xp: 0, stat_points: 0, atk: 0, def: 0,
        class: Some(PlayerClass::Tank), title: None, class_changed_at: None,
        hp_current: 100, hp_max: 100, hp_last_regen: None, repos_last_used: None,
        season: 1, created_at: now, updated_at: now,
    }
}

#[async_trait]
impl ManageCoudePlayersUseCase for MockPlayersUc {
    async fn get_or_create(&self, g: String, u: String, name: String) -> Result<Player, DomainError> {
        let mut p = make_player();
        p.guild_id = g;
        p.user_id = u;
        p.username = name;
        Ok(p)
    }
    async fn get(&self, _: &str, _: &str) -> Result<Player, DomainError> {
        self.player.lock().unwrap().clone()
            .ok_or_else(|| DomainError::NotFound("joueur".into()))
    }
    async fn list(&self, _: &str) -> Result<Vec<Player>, DomainError> { Ok(vec![]) }
    async fn random_active(&self, _: &str, _: i64) -> Result<Vec<Player>, DomainError> { Ok(vec![]) }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
    async fn update_class(&self, g: &str, u: &str, class: &str) -> Result<(), DomainError> {
        self.class_calls.lock().unwrap().push((g.into(), u.into(), class.into()));
        Ok(())
    }
    async fn add_xp(&self, g: &str, u: &str, a: i64) -> Result<XpProgress, DomainError> {
        self.add_xp_calls.lock().unwrap().push((g.into(), u.into(), a));
        Ok(XpProgress { new_xp: a, new_level: 2, leveled_up: true, stat_points_gained: 1 })
    }
    async fn spend_stat_point(&self, _: &str, _: &str, _: CombatStat) -> Result<Player, DomainError> { unimplemented!() }
    async fn reset_stats(&self, _: &str, _: &str, _: i64) -> Result<Player, DomainError> { unimplemented!() }
    async fn record_win(&self, _: &str, _: &str, _: i64, _: i64) -> Result<(), DomainError> { Ok(()) }
    async fn record_loss(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { Ok(()) }
    async fn record_draw(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { Ok(()) }
    async fn increment_cowardice(&self, _: &str, _: &str) -> Result<i32, DomainError> { Ok(1) }
    async fn increment_chaos(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn record_coins_earned(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { Ok(()) }
    async fn record_coins_lost(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { Ok(()) }
    async fn update_hp(&self, g: &str, u: &str, cur: i32, max: i32) -> Result<(), DomainError> {
        self.update_hp_calls.lock().unwrap().push((g.into(), u.into(), cur, max));
        Ok(())
    }
    async fn full_heal(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn regen_hp_tick(&self, _: f64, _: f64, _: f64, _: f64) -> Result<u64, DomainError> {
        Ok(*self.regen_return.lock().unwrap())
    }
}

#[derive(Default)]
struct MockWalletUc {
    credit_calls: Mutex<Vec<(String, String, i64)>>,
    debit_calls: Mutex<Vec<(String, String, i64)>>,
}

#[async_trait]
impl ManageWalletUseCase for MockWalletUc {
    async fn credit(&self, g: &str, u: &str, a: i64, _: &str, _: &str) -> Result<WalletMutation, DomainError> {
        self.credit_calls.lock().unwrap().push((g.into(), u.into(), a));
        Ok(WalletMutation { new_balance: a, previous_balance: 0, triggered_taunts: vec![] })
    }
    async fn debit(&self, g: &str, u: &str, a: i64, _: &str, _: &str) -> Result<WalletMutation, DomainError> {
        self.debit_calls.lock().unwrap().push((g.into(), u.into(), a));
        Ok(WalletMutation { new_balance: 0, previous_balance: a, triggered_taunts: vec![] })
    }
    async fn transfer(&self, _: &str, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<Vec<TauntEvent>, DomainError> { Ok(vec![]) }
    async fn get_balance(&self, _: &str, _: &str) -> Result<i64, DomainError> { Ok(0) }
    async fn credit_tx(&self, _: &mut Transaction<'_, Postgres>, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<TxWalletMutation, DomainError> { unimplemented!() }
    async fn debit_tx(&self, _: &mut Transaction<'_, Postgres>, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<TxWalletMutation, DomainError> { unimplemented!() }
    async fn post_commit_taunts(&self, _: &str, _: &str, _: &TxWalletMutation) -> Vec<TauntEvent> { vec![] }
}

fn grpc(players: Arc<MockPlayersUc>, wallet: Arc<MockWalletUc>) -> PlayerGrpc {
    PlayerGrpc { players_uc: players, wallet_uc: wallet }
}

// ── Tests ──

#[tokio::test]
async fn get_or_create_player_delegates() {
    let g = grpc(Arc::new(MockPlayersUc::default()), Arc::new(MockWalletUc::default()));
    let resp = g.get_or_create_player(Request::new(proto::GetOrCreatePlayerRequest {
        guild_id: "g1".into(), user_id: "u1".into(), username: "Alice".into(),
    })).await.unwrap();
    let p = resp.into_inner();
    assert_eq!(p.guild_id, "g1");
    assert_eq!(p.user_id, "u1");
    assert_eq!(p.username, "Alice");
}

#[tokio::test]
async fn get_player_success() {
    let players = Arc::new(MockPlayersUc::default());
    *players.player.lock().unwrap() = Some(make_player());
    let g = grpc(players, Arc::new(MockWalletUc::default()));
    let resp = g.get_player(Request::new(proto::GetPlayerRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap();
    assert_eq!(resp.into_inner().user_id, "u");
}

#[tokio::test]
async fn get_player_not_found() {
    let g = grpc(Arc::new(MockPlayersUc::default()), Arc::new(MockWalletUc::default()));
    let err = g.get_player(Request::new(proto::GetPlayerRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn update_player_class_rejects_invalid_class() {
    let g = grpc(Arc::new(MockPlayersUc::default()), Arc::new(MockWalletUc::default()));
    let err = g.update_player_class(Request::new(proto::UpdatePlayerClassRequest {
        guild_id: "g".into(), user_id: "u".into(),
        class: "wizard".into(),
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("inconnue"));
}

#[tokio::test]
async fn update_player_class_accepts_valid_class() {
    let players = Arc::new(MockPlayersUc::default());
    let g = grpc(players.clone(), Arc::new(MockWalletUc::default()));
    let _ = g.update_player_class(Request::new(proto::UpdatePlayerClassRequest {
        guild_id: "g".into(), user_id: "u".into(),
        class: "tank".into(),
    })).await.unwrap();
    let calls = players.class_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].2, "tank");
}

#[tokio::test]
async fn add_xp_returns_progress() {
    let players = Arc::new(MockPlayersUc::default());
    let g = grpc(players.clone(), Arc::new(MockWalletUc::default()));
    let resp = g.add_xp(Request::new(proto::AddXpRequest {
        guild_id: "g".into(), user_id: "u".into(), amount: 100,
    })).await.unwrap();
    let p = resp.into_inner();
    assert_eq!(p.new_xp, 100);
    assert_eq!(p.new_level, 2);
    assert!(p.leveled_up);
    assert_eq!(p.stat_points_gained, 1);

    assert_eq!(players.add_xp_calls.lock().unwrap()[0].2, 100);
}

#[tokio::test]
async fn adjust_coins_positive_credits_wallet() {
    let wallet = Arc::new(MockWalletUc::default());
    let g = grpc(Arc::new(MockPlayersUc::default()), wallet.clone());
    let _ = g.adjust_coins(Request::new(proto::AdjustCoinsRequest {
        guild_id: "g".into(), user_id: "u".into(), delta: 500,
    })).await.unwrap();
    let credits = wallet.credit_calls.lock().unwrap();
    assert_eq!(credits.len(), 1);
    assert_eq!(credits[0].2, 500);
    assert!(wallet.debit_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn adjust_coins_negative_debits_wallet_with_absolute_value() {
    let wallet = Arc::new(MockWalletUc::default());
    let g = grpc(Arc::new(MockPlayersUc::default()), wallet.clone());
    let _ = g.adjust_coins(Request::new(proto::AdjustCoinsRequest {
        guild_id: "g".into(), user_id: "u".into(), delta: -250,
    })).await.unwrap();
    let debits = wallet.debit_calls.lock().unwrap();
    assert_eq!(debits.len(), 1);
    assert_eq!(debits[0].2, 250); // valeur absolue
    assert!(wallet.credit_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn adjust_coins_zero_is_noop() {
    let wallet = Arc::new(MockWalletUc::default());
    let g = grpc(Arc::new(MockPlayersUc::default()), wallet.clone());
    let _ = g.adjust_coins(Request::new(proto::AdjustCoinsRequest {
        guild_id: "g".into(), user_id: "u".into(), delta: 0,
    })).await.unwrap();
    assert!(wallet.credit_calls.lock().unwrap().is_empty());
    assert!(wallet.debit_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn update_hp_delegates_to_uc() {
    let players = Arc::new(MockPlayersUc::default());
    let g = grpc(players.clone(), Arc::new(MockWalletUc::default()));
    let _ = g.update_hp(Request::new(proto::UpdateHpRequest {
        guild_id: "g".into(), user_id: "u".into(),
        hp_current: 50, hp_max: 100,
    })).await.unwrap();
    let calls = players.update_hp_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], ("g".into(), "u".into(), 50, 100));
}

#[tokio::test]
async fn hp_regen_tick_returns_updated_count() {
    let players = Arc::new(MockPlayersUc::default());
    *players.regen_return.lock().unwrap() = 42;
    let g = grpc(players, Arc::new(MockWalletUc::default()));
    let resp = g.hp_regen_tick(Request::new(proto::HpRegenTickRequest {
        rate_0_25: 1.0, rate_25_50: 2.0, rate_50_75: 3.0, rate_75_100: 4.0,
    })).await.unwrap();
    assert_eq!(resp.into_inner().updated, 42);
}

// Avoid unused import warnings
#[allow(dead_code)]
fn _unused() {
    let _ = Uuid::new_v4();
}
