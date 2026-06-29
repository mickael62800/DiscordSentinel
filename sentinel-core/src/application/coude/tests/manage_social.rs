//! Tests de ManageCoudeSocialService. Focus sur les validations + early
//! returns de trigger_daily_chaos. Les chemins full-success qui appellent
//! wallet_uc.transfer sont testes via l'integration wallet et ne sont pas
//! duplique ici.

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::application::coude::manage_social_service::ManageCoudeSocialService;
use crate::domain::entities::coude::player::CombatStat;
use crate::domain::entities::coude::player::Player;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::entities::coude::social::Event;
use crate::domain::entities::coude::social::LeaderboardCategory;
use crate::domain::entities::coude::social::LeaderboardEntry;
use crate::domain::entities::coude::social::NewDailyChaos;
use crate::domain::entities::coude::social::Season;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::entities::system::bot_config::BotDefinition;
use crate::domain::entities::system::bot_config::BotGuildConfig;
use crate::domain::enums::coude::coude_class::PlayerClass;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::casino::manage_wallet::TxWalletMutation;
use crate::ports::inbound::casino::manage_wallet::WalletMutation;
use crate::ports::inbound::coude::manage_social::ManageCoudeSocialUseCase;
use crate::ports::outbound::coude::economy_repository::EconomyRepository;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::coude::social_repository::SocialRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use sqlx::Postgres;
use sqlx::Transaction;
// ── Mock SocialRepository ──

#[derive(Default)]
struct MockSocialRepo {
    cooldown_returns: Mutex<Option<DateTime<Utc>>>,
    set_cooldown_calls: Mutex<Vec<(String, String, String, i64)>>,
    leaderboard_returns: Mutex<Vec<LeaderboardEntry>>,
    leaderboard_limit_calls: Mutex<Vec<i64>>,
    active_events: Mutex<Vec<Event>>,
    daily_chaos_count: Mutex<i64>,
    daily_chaos_logs: Mutex<Vec<NewDailyChaos>>,
    season: Mutex<Option<Season>>,
}

#[async_trait]
impl SocialRepository for MockSocialRepo {
    async fn get_cooldown(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        Ok(*self.cooldown_returns.lock().unwrap())
    }
    async fn set_cooldown(&self, g: &str, u: &str, a: &str, d: i64) -> Result<(), DomainError> {
        self.set_cooldown_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), a.into(), d));
        Ok(())
    }
    async fn leaderboard(
        &self,
        _: &str,
        _: LeaderboardCategory,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, DomainError> {
        self.leaderboard_limit_calls.lock().unwrap().push(limit);
        Ok(self.leaderboard_returns.lock().unwrap().clone())
    }
    async fn list_active_events(&self, _: &str) -> Result<Vec<Event>, DomainError> {
        Ok(self.active_events.lock().unwrap().clone())
    }
    async fn log_daily_chaos(&self, chaos: NewDailyChaos) -> Result<(), DomainError> {
        self.daily_chaos_logs.lock().unwrap().push(chaos);
        Ok(())
    }
    async fn count_daily_chaos_today(&self, _: &str) -> Result<i64, DomainError> {
        Ok(*self.daily_chaos_count.lock().unwrap())
    }
    async fn get_or_bootstrap_current_season(
        &self,
        _guild_id: &str,
    ) -> Result<Season, DomainError> {
        Ok(self.season.lock().unwrap().clone().unwrap_or(Season {
            season_number: 1,
            started_at: Utc::now(),
            ends_at: Utc::now(),
            days_remaining: 30,
        }))
    }
}

// ── Mock PlayerRepository (minimal — seul random_active est exerce) ──

#[derive(Default)]
struct MockPlayerRepo {
    random_returns: Mutex<Vec<Player>>,
}

#[async_trait]
impl PlayerRepository for MockPlayerRepo {
    async fn get_or_create(&self, _: &str, _: &str, _: &str) -> Result<Player, DomainError> {
        unimplemented!()
    }
    async fn get(&self, _: &str, _: &str) -> Result<Option<Player>, DomainError> {
        Ok(None)
    }
    async fn list(&self, _: &str, _: i64) -> Result<Vec<Player>, DomainError> {
        Ok(vec![])
    }
    async fn random_active(&self, _: &str, _: i64, _: i64) -> Result<Vec<Player>, DomainError> {
        Ok(self.random_returns.lock().unwrap().clone())
    }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> {
        Ok(vec![])
    }
    async fn update_class(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn add_xp(&self, _: &str, _: &str, _: i64) -> Result<Option<XpProgress>, DomainError> {
        Ok(None)
    }
    async fn spend_stat_point(
        &self,
        _: &str,
        _: &str,
        _: CombatStat,
    ) -> Result<Option<Player>, DomainError> {
        Ok(None)
    }
    async fn reset_stats(&self, _: &str, _: &str, _: i64) -> Result<Option<Player>, DomainError> {
        Ok(None)
    }
    async fn record_coins_earned(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn record_coins_lost(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn record_win(&self, _: &str, _: &str, _: i64, _: i64) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn record_loss(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn record_draw(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn touch_win_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }
    async fn touch_loss_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }
    async fn reset_combat_streaks(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn touch_steal_victim_streak(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }
    async fn reset_steal_victim_streak(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn touch_bj_win_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }
    async fn touch_bj_bust_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }
    async fn reset_bj_bust_streak(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn increment_cowardice(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }
    async fn increment_chaos(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn update_hp(&self, _: &str, _: &str, _: i32, _: i32) -> Result<(), DomainError> {
        Ok(())
    }
    async fn full_heal(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn regen_hp_tick(&self, _: f64, _: f64, _: f64, _: f64) -> Result<u64, DomainError> {
        Ok(0)
    }
}

// ── Mock EconomyRepository ──

#[derive(Default)]
struct MockEconomyRepo {
    steal_calls: Mutex<Vec<(String, String, String, i64)>>,
}

#[async_trait]
impl EconomyRepository for MockEconomyRepo {
    async fn record_steal_stats(
        &self,
        g: &str,
        thief: &str,
        victim: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        self.steal_calls
            .lock()
            .unwrap()
            .push((g.into(), thief.into(), victim.into(), amount));
        Ok(())
    }
    async fn record_steal_fail_stats(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_coins(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(0)
    }
    async fn record_casino_win_stats(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn record_casino_loss_stats(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn record_casino_faillite_stats(
        &self,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<i64, DomainError> {
        Ok(0)
    }
    async fn count_casino_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(0)
    }
    async fn sum_casino_gains_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(0)
    }
    async fn count_steal_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(0)
    }
}

// ── Mock BotConfigRepository ──

#[derive(Default)]
struct MockBotConfig {
    config_rows: Mutex<Vec<BotGuildConfig>>,
}

impl MockBotConfig {
    fn with_kv(self, key: &str, value: &str) -> Self {
        self.config_rows.lock().unwrap().push(BotGuildConfig {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            bot_name: "coude-bot".into(),
            config_key: key.into(),
            config_value: value.into(),
            updated_at: Utc::now(),
        });
        self
    }
}

#[async_trait]
impl BotConfigRepository for MockBotConfig {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
        Ok(vec![])
    }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(self.config_rows.lock().unwrap().clone())
    }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

// ── Mock ManageWalletUseCase ──

#[derive(Default)]
struct MockWalletUc {
    transfer_calls: Mutex<Vec<(String, String, String, i64)>>,
}

#[async_trait]
impl ManageWalletUseCase for MockWalletUc {
    async fn credit(
        &self,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<WalletMutation, DomainError> {
        unimplemented!()
    }
    async fn debit(
        &self,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<WalletMutation, DomainError> {
        unimplemented!()
    }
    async fn transfer(
        &self,
        g: &str,
        from: &str,
        to: &str,
        amount: i64,
        _: &str,
        _: &str,
    ) -> Result<Vec<TauntEvent>, DomainError> {
        self.transfer_calls
            .lock()
            .unwrap()
            .push((g.into(), from.into(), to.into(), amount));
        Ok(vec![])
    }
    async fn get_balance(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(0)
    }
    async fn credit_tx(
        &self,
        _: &mut dyn crate::ports::uow::DbTx,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<TxWalletMutation, DomainError> {
        unimplemented!()
    }
    async fn debit_tx(
        &self,
        _: &mut dyn crate::ports::uow::DbTx,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<TxWalletMutation, DomainError> {
        unimplemented!()
    }
    async fn post_commit_taunts(&self, _: &str, _: &str, _: &TxWalletMutation) -> Vec<TauntEvent> {
        vec![]
    }
}

// ── Helpers ──

fn make_player(user_id: &str, coins: i64) -> Player {
    let now = Utc::now();
    Player {
        guild_id: "g".into(),
        user_id: user_id.into(),
        username: format!("user_{user_id}"),
        coins,
        total_wins: 0,
        total_losses: 0,
        total_draws: 0,
        total_earned: 0,
        total_lost: 0,
        total_stolen: 0,
        cowardice_count: 0,
        chaos_events: 0,
        casino_wins: 0,
        casino_losses: 0,
        level: 1,
        xp: 0,
        stat_points: 0,
        atk: 0,
        def: 0,
        class: Some(PlayerClass::Tank),
        title: None,
        class_changed_at: None,
        hp_current: 100,
        hp_max: 100,
        hp_last_regen: None,
        repos_last_used: None,
        season: 1,
        created_at: now,
        updated_at: now,
    }
}

fn build_service(
    social: Arc<MockSocialRepo>,
    player: Arc<MockPlayerRepo>,
    economy: Arc<MockEconomyRepo>,
    bot_config: Arc<MockBotConfig>,
    wallet: Arc<MockWalletUc>,
) -> ManageCoudeSocialService {
    ManageCoudeSocialService::new(social, player, economy, bot_config, wallet)
}

// ═══════════════════════════════════════════════════════════════════
// set_cooldown — validation
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn set_cooldown_rejects_zero_duration() {
    let social = Arc::new(MockSocialRepo::default());
    let svc = build_service(
        social.clone(),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockEconomyRepo::default()),
        Arc::new(MockBotConfig::default()),
        Arc::new(MockWalletUc::default()),
    );
    let err = svc.set_cooldown("g", "u", "action", 0).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
    assert!(social.set_cooldown_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn set_cooldown_rejects_negative_duration() {
    let social = Arc::new(MockSocialRepo::default());
    let svc = build_service(
        social.clone(),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockEconomyRepo::default()),
        Arc::new(MockBotConfig::default()),
        Arc::new(MockWalletUc::default()),
    );
    let err = svc.set_cooldown("g", "u", "a", -10).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn set_cooldown_valid_delegates_to_repo() {
    let social = Arc::new(MockSocialRepo::default());
    let svc = build_service(
        social.clone(),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockEconomyRepo::default()),
        Arc::new(MockBotConfig::default()),
        Arc::new(MockWalletUc::default()),
    );
    svc.set_cooldown("g1", "u1", "steal", 300).await.unwrap();
    let calls = social.set_cooldown_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], ("g1".into(), "u1".into(), "steal".into(), 300));
}

// ═══════════════════════════════════════════════════════════════════
// check_cooldown — delegate to repo
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn check_cooldown_returns_repo_value() {
    let expiry = Utc::now() + chrono::Duration::minutes(5);
    let social = Arc::new(MockSocialRepo::default());
    *social.cooldown_returns.lock().unwrap() = Some(expiry);
    let svc = build_service(
        social,
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockEconomyRepo::default()),
        Arc::new(MockBotConfig::default()),
        Arc::new(MockWalletUc::default()),
    );
    let got = svc.check_cooldown("g", "u", "a").await.unwrap();
    assert_eq!(got, Some(expiry));
}

// ═══════════════════════════════════════════════════════════════════
// leaderboard — clamp limit
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn leaderboard_clamps_oversized_limit() {
    let social = Arc::new(MockSocialRepo::default());
    let svc = build_service(
        social.clone(),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockEconomyRepo::default()),
        Arc::new(MockBotConfig::default()),
        Arc::new(MockWalletUc::default()),
    );
    svc.leaderboard("g", LeaderboardCategory::Richest, 9999)
        .await
        .unwrap();
    let calls = social.leaderboard_limit_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    // clamp_leaderboard_limit ramène 9999 vers LEADERBOARD_MAX_LIMIT
    assert!(calls[0] < 9999);
    assert!(calls[0] > 0);
}

#[tokio::test]
async fn leaderboard_clamps_negative_limit() {
    let social = Arc::new(MockSocialRepo::default());
    let svc = build_service(
        social.clone(),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockEconomyRepo::default()),
        Arc::new(MockBotConfig::default()),
        Arc::new(MockWalletUc::default()),
    );
    svc.leaderboard("g", LeaderboardCategory::Richest, -5)
        .await
        .unwrap();
    let calls = social.leaderboard_limit_calls.lock().unwrap();
    assert!(calls[0] >= 1);
}

// ═══════════════════════════════════════════════════════════════════
// trigger_daily_chaos — early returns
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn trigger_daily_chaos_returns_none_when_cap_reached() {
    let social = Arc::new(MockSocialRepo::default());
    *social.daily_chaos_count.lock().unwrap() = 999; // > DAILY_CHAOS_MAX
    let svc = build_service(
        social,
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockEconomyRepo::default()),
        Arc::new(MockBotConfig::default()),
        Arc::new(MockWalletUc::default()),
    );
    let got = svc.trigger_daily_chaos("g").await.unwrap();
    assert!(got.is_none());
}

#[tokio::test]
async fn trigger_daily_chaos_returns_none_when_channel_unset() {
    let social = Arc::new(MockSocialRepo::default());
    *social.daily_chaos_count.lock().unwrap() = 0;
    // MockBotConfig default: aucun channel_announcements
    let svc = build_service(
        social,
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockEconomyRepo::default()),
        Arc::new(MockBotConfig::default()),
        Arc::new(MockWalletUc::default()),
    );
    let got = svc.trigger_daily_chaos("g").await.unwrap();
    assert!(got.is_none());
}

#[tokio::test]
async fn trigger_daily_chaos_returns_none_when_channel_empty_string() {
    let social = Arc::new(MockSocialRepo::default());
    let cfg = MockBotConfig::default().with_kv("channel_announcements", "");
    let svc = build_service(
        social,
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockEconomyRepo::default()),
        Arc::new(cfg),
        Arc::new(MockWalletUc::default()),
    );
    let got = svc.trigger_daily_chaos("g").await.unwrap();
    assert!(got.is_none());
}

#[tokio::test]
async fn trigger_daily_chaos_returns_none_when_not_enough_players() {
    let social = Arc::new(MockSocialRepo::default());
    let player = Arc::new(MockPlayerRepo::default());
    *player.random_returns.lock().unwrap() = vec![make_player("alone", 5000)]; // only 1
    let cfg = MockBotConfig::default().with_kv("channel_announcements", "c1");
    let svc = build_service(
        social,
        player,
        Arc::new(MockEconomyRepo::default()),
        Arc::new(cfg),
        Arc::new(MockWalletUc::default()),
    );
    let got = svc.trigger_daily_chaos("g").await.unwrap();
    assert!(got.is_none());
}

#[tokio::test]
async fn trigger_daily_chaos_success_path_transfers_and_logs() {
    let social = Arc::new(MockSocialRepo::default());
    let player = Arc::new(MockPlayerRepo::default());
    *player.random_returns.lock().unwrap() =
        vec![make_player("victim", 10_000), make_player("winner", 0)];
    let cfg = MockBotConfig::default().with_kv("channel_announcements", "channel-1");
    let economy = Arc::new(MockEconomyRepo::default());
    let wallet = Arc::new(MockWalletUc::default());
    let svc = build_service(
        social.clone(),
        player,
        economy.clone(),
        Arc::new(cfg),
        wallet.clone(),
    );
    let got = svc.trigger_daily_chaos("g1").await.unwrap();
    let outcome = got.expect("chaos should trigger");
    assert_eq!(outcome.loser_id, "victim");
    assert_eq!(outcome.winner_id, "winner");
    assert_eq!(outcome.channel_id, "channel-1");
    assert!(outcome.amount > 0);

    // Verifier les appels side-effects
    let transfer_calls = wallet.transfer_calls.lock().unwrap();
    assert_eq!(transfer_calls.len(), 1);
    assert_eq!(transfer_calls[0].1, "victim");
    assert_eq!(transfer_calls[0].2, "winner");

    let steal_calls = economy.steal_calls.lock().unwrap();
    assert_eq!(steal_calls.len(), 1);

    let logs = social.daily_chaos_logs.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].winner_id, "winner");
    assert_eq!(logs[0].loser_id, "victim");
}

#[tokio::test]
async fn trigger_daily_chaos_custom_percent_config() {
    let social = Arc::new(MockSocialRepo::default());
    let player = Arc::new(MockPlayerRepo::default());
    *player.random_returns.lock().unwrap() = vec![make_player("v", 10_000), make_player("w", 0)];
    // 50% au lieu du default 20%
    let cfg = MockBotConfig::default()
        .with_kv("channel_announcements", "c")
        .with_kv("daily_chaos_percent", "50");
    let wallet = Arc::new(MockWalletUc::default());
    let svc = build_service(
        social,
        player,
        Arc::new(MockEconomyRepo::default()),
        Arc::new(cfg),
        wallet.clone(),
    );
    let outcome = svc.trigger_daily_chaos("g").await.unwrap().unwrap();
    // 50% de 10_000 = 5000
    assert_eq!(outcome.amount, 5000);
}

// ═══════════════════════════════════════════════════════════════════
// list_active_events / log_daily_chaos / current_season — delegates
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn list_active_events_returns_repo_value() {
    let social = Arc::new(MockSocialRepo::default());
    social.active_events.lock().unwrap().push(Event {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        event_type: "happy_hour".into(),
        active: true,
        expires_at: Utc::now(),
        created_at: Utc::now(),
    });
    let svc = build_service(
        social,
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockEconomyRepo::default()),
        Arc::new(MockBotConfig::default()),
        Arc::new(MockWalletUc::default()),
    );
    let events = svc.list_active_events("g").await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "happy_hour");
}

#[tokio::test]
async fn log_daily_chaos_delegates_to_repo() {
    let social = Arc::new(MockSocialRepo::default());
    let svc = build_service(
        social.clone(),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockEconomyRepo::default()),
        Arc::new(MockBotConfig::default()),
        Arc::new(MockWalletUc::default()),
    );
    svc.log_daily_chaos(NewDailyChaos {
        guild_id: "g".into(),
        loser_id: "v".into(),
        loser_name: "V".into(),
        winner_id: "w".into(),
        winner_name: "W".into(),
        amount: 500,
    })
    .await
    .unwrap();
    let logs = social.daily_chaos_logs.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].amount, 500);
}

#[tokio::test]
async fn current_season_delegates_to_repo() {
    let social = Arc::new(MockSocialRepo::default());
    *social.season.lock().unwrap() = Some(Season {
        season_number: 7,
        started_at: Utc::now(),
        ends_at: Utc::now(),
        days_remaining: 10,
    });
    let svc = build_service(
        social,
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockEconomyRepo::default()),
        Arc::new(MockBotConfig::default()),
        Arc::new(MockWalletUc::default()),
    );
    let s = svc.current_season("g").await.unwrap();
    assert_eq!(s.season_number, 7);
    assert_eq!(s.days_remaining, 10);
}
