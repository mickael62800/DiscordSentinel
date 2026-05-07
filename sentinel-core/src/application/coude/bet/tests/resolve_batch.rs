//! Tests d'orchestration pour ResolveBettingBatchService.
//!
//! Focus: wire-up du flow batch (claim → resolve_one × N).
//! Les règles métier (insurance, XP, formatage) sont testées en pur
//! dans combat_resolution_rules.

use std::sync::Arc;
use std::sync::Mutex;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::application::coude::bet::resolve_batch::ResolveBettingBatchService;
use crate::domain::entities::system::bot_config::BotDefinition;
use crate::domain::entities::system::bot_config::BotGuildConfig;
use crate::domain::entities::coude::combat::CombatResolution;
use crate::domain::entities::coude::player::CombatStat;
use crate::domain::entities::coude::bet::Bet;
use crate::domain::entities::coude::combat::Combat;
use crate::domain::entities::coude::social::Season;
use crate::domain::entities::coude::social::Event;
use crate::domain::entities::coude::inventory::Insurance;
use crate::domain::entities::coude::inventory::InventoryItem;
use crate::domain::entities::coude::social::LeaderboardEntry;
use crate::domain::entities::coude::player::Player;
use crate::domain::entities::coude::inventory::Prime;
use crate::domain::entities::coude::taunt::TauntsConfig;
use crate::domain::entities::coude::social::DailyChaosOutcome;
use crate::domain::entities::coude::social::LeaderboardCategory;
use crate::domain::entities::coude::bet::NewCoudeBet;
use crate::domain::entities::coude::combat::NewCoudeCombat;
use crate::domain::entities::coude::inventory::NewCoudePrime;
use crate::domain::entities::coude::social::NewDailyChaos;
use crate::domain::entities::coude::bet::RefundSummary;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::entities::casino::wallet::Wallet;
use crate::domain::entities::casino::wallet::WalletTransaction;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::errors::DomainError;
use crate::domain::enums::coude::coude_class::PlayerClass;
use crate::ports::inbound::coude::manage_bets::ManageCoudeBetsUseCase;
use crate::ports::inbound::coude::manage_bets::PlaceBetOutcome;
use crate::ports::inbound::coude::manage_bets::ResolveBetsOutcome;
use crate::ports::inbound::coude::manage_social::ManageCoudeSocialUseCase;
use crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use crate::ports::inbound::coude::resolve_betting_batch::ResolveBettingBatchUseCase;
use crate::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::coude::combat_repository::CombatRepository;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use chrono::DateTime;

// ══════════════════════════════════════════════════════════════════════
// Mocks
// ══════════════════════════════════════════════════════════════════════

#[derive(Default)]
struct MockCombatRepo {
    due: Mutex<Vec<Combat>>,
    stuck: Mutex<Vec<Combat>>,
    resolve_calls: Mutex<Vec<(Uuid, CombatResolution)>>,
}

#[async_trait]
impl CombatRepository for MockCombatRepo {
    async fn list(&self, _: &str, _: Option<&str>, _: i64) -> Result<Vec<Combat>, DomainError> { Ok(vec![]) }
    async fn get(&self, _: Uuid) -> Result<Option<Combat>, DomainError> { Ok(None) }
    async fn get_pending_for_attacker(&self, _: &str, _: &str) -> Result<Option<Combat>, DomainError> { Ok(None) }
    async fn get_pending_for_defender(&self, _: &str, _: &str) -> Result<Option<Combat>, DomainError> { Ok(None) }
    async fn list_expired_pending(&self) -> Result<Vec<Combat>, DomainError> { Ok(vec![]) }
    async fn claim_due_betting_combats(&self, _: i64) -> Result<Vec<Combat>, DomainError> {
        Ok(std::mem::take(&mut *self.due.lock().unwrap()))
    }
    async fn claim_stuck_resolving_combats(&self, _: i64) -> Result<Vec<Combat>, DomainError> {
        Ok(std::mem::take(&mut *self.stuck.lock().unwrap()))
    }
    async fn claim_expired_pending_combats(&self, _: i64) -> Result<Vec<Combat>, DomainError> { Ok(vec![]) }
    async fn get_betting_for_participant(&self, _: &str, _: &str) -> Result<Option<Combat>, DomainError> { Ok(None) }
    async fn create(&self, _: NewCoudeCombat) -> Result<Combat, DomainError> { unimplemented!() }
    async fn resolve(&self, id: Uuid, r: CombatResolution) -> Result<bool, DomainError> {
        self.resolve_calls.lock().unwrap().push((id, r));
        Ok(true)
    }
    async fn set_betting(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn expire(&self, _: Uuid) -> Result<bool, DomainError> { Ok(true) }
    async fn cancel_pending(&self, _: Uuid) -> Result<bool, DomainError> { Ok(true) }
    async fn set_defender_special(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn mark_unresolved_bets_lost(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
}

#[derive(Default)]
struct MockPlayerRepo {
    players: Mutex<std::collections::HashMap<String, Player>>,
    add_xp_calls: Mutex<Vec<(String, String, i64)>>,
    update_hp_calls: Mutex<Vec<(String, String, i32, i32)>>,
    record_win_calls: Mutex<Vec<(String, String, i64, i64)>>,
    record_loss_calls: Mutex<Vec<(String, String, i64)>>,
    record_draw_calls: Mutex<Vec<(String, String, i64)>>,
}

impl MockPlayerRepo {
    fn insert(&self, p: Player) {
        self.players.lock().unwrap().insert(format!("{}:{}", p.guild_id, p.user_id), p);
    }
}

#[async_trait]
impl PlayerRepository for MockPlayerRepo {
    async fn get_or_create(&self, _: &str, _: &str, _: &str) -> Result<Player, DomainError> { unimplemented!() }
    async fn get(&self, g: &str, u: &str) -> Result<Option<Player>, DomainError> {
        Ok(self.players.lock().unwrap().get(&format!("{g}:{u}")).cloned())
    }
    async fn list(&self, _: &str, _: i64) -> Result<Vec<Player>, DomainError> { Ok(vec![]) }
    async fn random_active(&self, _: &str, _: i64, _: i64) -> Result<Vec<Player>, DomainError> { Ok(vec![]) }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
    async fn update_class(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn add_xp(&self, g: &str, u: &str, a: i64) -> Result<Option<XpProgress>, DomainError> {
        self.add_xp_calls.lock().unwrap().push((g.into(), u.into(), a));
        Ok(Some(XpProgress { new_xp: a, new_level: 1, leveled_up: false, stat_points_gained: 0 }))
    }
    async fn spend_stat_point(&self, _: &str, _: &str, _: CombatStat) -> Result<Option<Player>, DomainError> { Ok(None) }
    async fn reset_stats(&self, _: &str, _: &str, _: i64) -> Result<Option<Player>, DomainError> { Ok(None) }
    async fn record_coins_earned(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn record_coins_lost(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn record_win(&self, g: &str, u: &str, e: i64, s: i64) -> Result<bool, DomainError> {
        self.record_win_calls.lock().unwrap().push((g.into(), u.into(), e, s));
        Ok(true)
    }
    async fn record_loss(&self, g: &str, u: &str, l: i64) -> Result<bool, DomainError> {
        self.record_loss_calls.lock().unwrap().push((g.into(), u.into(), l));
        Ok(true)
    }
    async fn record_draw(&self, g: &str, u: &str, l: i64) -> Result<bool, DomainError> {
        self.record_draw_calls.lock().unwrap().push((g.into(), u.into(), l));
        Ok(true)
    }
    async fn touch_win_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn touch_loss_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn reset_combat_streaks(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn touch_steal_victim_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn reset_steal_victim_streak(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn touch_bj_win_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn touch_bj_bust_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn reset_bj_bust_streak(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn increment_cowardice(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn increment_chaos(&self, _: &str, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn update_hp(&self, g: &str, u: &str, cur: i32, max: i32) -> Result<(), DomainError> {
        self.update_hp_calls.lock().unwrap().push((g.into(), u.into(), cur, max));
        Ok(())
    }
    async fn full_heal(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn regen_hp_tick(&self, _: f64, _: f64, _: f64, _: f64) -> Result<u64, DomainError> { Ok(0) }
}

#[derive(Default)]
struct MockWalletRepo {
    wallets: Mutex<std::collections::HashMap<String, i64>>,
    credit_calls: Mutex<Vec<(String, String, i64, String)>>,
    debit_calls: Mutex<Vec<(String, String, i64, String)>>,
}

impl MockWalletRepo {
    fn set_balance(&self, g: &str, u: &str, coins: i64) {
        self.wallets.lock().unwrap().insert(format!("{g}:{u}"), coins);
    }
}

#[async_trait]
impl WalletRepository for MockWalletRepo {
    async fn get_or_create(&self, _: &str, _: &str, _: &str, _: i64) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn get(&self, g: &str, u: &str) -> Result<Option<Wallet>, DomainError> {
        let map = self.wallets.lock().unwrap();
        Ok(map.get(&format!("{g}:{u}")).map(|&c| Wallet {
            id: Uuid::new_v4(), guild_id: g.into(), user_id: u.into(), username: u.into(),
            coins: c, total_earned: 0, total_spent: 0,
            created_at: Utc::now(), updated_at: Utc::now(),
        }))
    }
    async fn credit(&self, g: &str, u: &str, amount: i64, source: &str, _: &str) -> Result<Wallet, DomainError> {
        self.credit_calls.lock().unwrap().push((g.into(), u.into(), amount, source.into()));
        Ok(Wallet {
            id: Uuid::new_v4(), guild_id: g.into(), user_id: u.into(), username: u.into(),
            coins: amount, total_earned: amount, total_spent: 0,
            created_at: Utc::now(), updated_at: Utc::now(),
        })
    }
    async fn debit(&self, g: &str, u: &str, amount: i64, source: &str, _: &str) -> Result<Wallet, DomainError> {
        self.debit_calls.lock().unwrap().push((g.into(), u.into(), amount, source.into()));
        Ok(Wallet {
            id: Uuid::new_v4(), guild_id: g.into(), user_id: u.into(), username: u.into(),
            coins: 0, total_earned: 0, total_spent: amount,
            created_at: Utc::now(), updated_at: Utc::now(),
        })
    }
    async fn transfer(&self, _: &str, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn pay_combat_atomic(&self, _: &str, _: &str, _: i64, _: &str, _: i64, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn leaderboard(&self, _: &str, _: i64) -> Result<Vec<Wallet>, DomainError> { Ok(vec![]) }
    async fn get_transactions(&self, _: &str, _: &str, _: i64) -> Result<Vec<WalletTransaction>, DomainError> { Ok(vec![]) }
    async fn list_by_guild(&self, _: &str) -> Result<Vec<Wallet>, DomainError> { Ok(vec![]) }
    async fn reset_wallet(&self, _: &str, _: &str, _: i64) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn reset_all_wallets(&self, _: &str, _: i64) -> Result<u64, DomainError> { Ok(0) }
}

#[derive(Default)]
struct MockBetsUc {
    resolve_calls: Mutex<Vec<(Uuid, Option<String>)>>,
}

#[async_trait]
impl ManageCoudeBetsUseCase for MockBetsUc {
    async fn place(&self, _: NewCoudeBet) -> Result<PlaceBetOutcome, DomainError> { unimplemented!() }
    async fn list_for_combat(&self, _: Uuid) -> Result<Vec<Bet>, DomainError> { Ok(vec![]) }
    async fn resolve(&self, id: Uuid, winner: Option<String>) -> Result<ResolveBetsOutcome, DomainError> {
        self.resolve_calls.lock().unwrap().push((id, winner));
        Ok(ResolveBetsOutcome {
            plan: crate::domain::entities::coude::bet::BetResolutionPlan { payouts: vec![], fighter_bonus: None },
            taunt_events: vec![],
        })
    }
    async fn refund(&self, _: Uuid) -> Result<RefundSummary, DomainError> {
        Ok(RefundSummary { refunded_count: 0, refunded_total: 0 })
    }
}

#[derive(Default)]
struct MockInventoryUc {
    active_insurance: Mutex<Option<Insurance>>,
    primes_amount: Mutex<i64>,
    expire_calls: Mutex<Vec<Uuid>>,
}

#[async_trait]
impl ManageCoudeInventoryUseCase for MockInventoryUc {
    async fn list_inventory(&self, _: &str, _: &str) -> Result<Vec<InventoryItem>, DomainError> { Ok(vec![]) }
    async fn add_item(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn use_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn has_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn create_prime(&self, _: NewCoudePrime) -> Result<Prime, DomainError> { unimplemented!() }
    async fn list_active_primes(&self, _: &str, _: &str) -> Result<Vec<Prime>, DomainError> { Ok(vec![]) }
    async fn claim_primes(&self, _: &str, _: &str, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(*self.primes_amount.lock().unwrap())
    }
    async fn buy_insurance(&self, _: &str, _: &str, _: bool, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn get_active_insurance(&self, _: &str, _: &str) -> Result<Option<Insurance>, DomainError> {
        Ok(self.active_insurance.lock().unwrap().clone())
    }
    async fn expire_insurance(&self, id: Uuid) -> Result<(), DomainError> {
        self.expire_calls.lock().unwrap().push(id);
        Ok(())
    }
}

#[derive(Default)]
struct MockSocialUc {
    active_events: Mutex<Vec<Event>>,
}

#[async_trait]
impl ManageCoudeSocialUseCase for MockSocialUc {
    async fn check_cooldown(&self, _: &str, _: &str, _: &str) -> Result<Option<DateTime<Utc>>, DomainError> { Ok(None) }
    async fn set_cooldown(&self, _: &str, _: &str, _: &str, _: i64) -> Result<(), DomainError> { Ok(()) }
    async fn leaderboard(&self, _: &str, _: LeaderboardCategory, _: i64) -> Result<Vec<LeaderboardEntry>, DomainError> { Ok(vec![]) }
    async fn list_active_events(&self, _: &str) -> Result<Vec<Event>, DomainError> {
        Ok(self.active_events.lock().unwrap().clone())
    }
    async fn log_daily_chaos(&self, _: NewDailyChaos) -> Result<(), DomainError> { Ok(()) }
    async fn trigger_daily_chaos(&self, _: &str) -> Result<Option<DailyChaosOutcome>, DomainError> { Ok(None) }
    async fn current_season(&self, _: &str) -> Result<Season, DomainError> {
        Ok(Season { season_number: 1, started_at: Utc::now(), ends_at: Utc::now(), days_remaining: 30 })
    }
}

#[derive(Default)]
struct MockTauntsUc {
    won_calls: Mutex<Vec<(String, String)>>,
    lost_calls: Mutex<Vec<(String, String)>>,
}

#[async_trait]
impl ManageCoudeTauntsUseCase for MockTauntsUc {
    async fn on_player_won(&self, g: &str, u: &str) -> Result<Option<TauntEvent>, DomainError> {
        self.won_calls.lock().unwrap().push((g.into(), u.into()));
        Ok(None)
    }
    async fn on_player_lost(&self, g: &str, u: &str) -> Result<Option<TauntEvent>, DomainError> {
        self.lost_calls.lock().unwrap().push((g.into(), u.into()));
        Ok(None)
    }
    async fn on_player_drew(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn on_player_stolen_from(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_defended_steal(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn on_bj_natural(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_bj_hand_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_bj_hand_bust(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_bankruptcy(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_jackpot(&self, _: &str, _: &str, _: i64) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_generous_donor(&self, _: &str, _: &str, _: i64) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn get_config(&self, g: &str) -> Result<TauntsConfig, DomainError> {
        Ok(TauntsConfig { guild_id: g.into(), channel_id: None, enabled: true, rename_enabled: true, messages_enabled: true })
    }
    async fn set_channel(&self, _: &str, _: Option<&str>) -> Result<(), DomainError> { Ok(()) }
    async fn set_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn set_rename_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn set_messages_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn set_opt_out(&self, _: &str, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn is_opted_out(&self, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn list_opt_outs(&self, _: &str) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
}

#[derive(Default)]
struct MockBotConfig;

#[async_trait]
impl BotConfigRepository for MockBotConfig {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> { Ok(vec![]) }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> { Ok(vec![]) }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> { Ok(vec![]) }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
}

// ══════════════════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════════════════

fn make_player(user_id: &str, level: i32, hp: i32) -> Player {
    let now = Utc::now();
    Player {
        guild_id: "g".into(),
        user_id: user_id.into(),
        username: format!("user_{user_id}"),
        coins: 0,
        total_wins: 0, total_losses: 0, total_draws: 0,
        total_earned: 0, total_lost: 0, total_stolen: 0,
        cowardice_count: 0, chaos_events: 0, casino_wins: 0, casino_losses: 0,
        level, xp: 0, stat_points: 0, atk: 0, def: 0,
        class: Some(PlayerClass::Tank), title: None, class_changed_at: None,
        hp_current: hp, hp_max: 100, hp_last_regen: None, repos_last_used: None,
        season: 1, created_at: now, updated_at: now,
    }
}

fn make_combat(attacker_id: &str, defender_id: &str, mise: i64) -> Combat {
    Combat {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: Some("c1".into()),
        attacker_id: attacker_id.into(), attacker_name: format!("Atk_{attacker_id}"),
        defender_id: defender_id.into(), defender_name: format!("Def_{defender_id}"),
        mise,
        status: "betting".into(),
        winner_id: None,
        attacker_roll: None, defender_roll: None,
        chaos_event: None, special_attack: None, defender_special: None,
        coins_transferred: None, result_message: None, message_id: Some("msg1".into()),
        created_at: Utc::now(), accepted_at: Some(Utc::now()), resolved_at: None,
    }
}

#[allow(clippy::type_complexity)]
fn build_service() -> (
    ResolveBettingBatchService,
    Arc<MockCombatRepo>,
    Arc<MockPlayerRepo>,
    Arc<MockWalletRepo>,
    Arc<MockBetsUc>,
    Arc<MockTauntsUc>,
) {
    let combat_repo = Arc::new(MockCombatRepo::default());
    let player_repo = Arc::new(MockPlayerRepo::default());
    let wallet_repo = Arc::new(MockWalletRepo::default());
    let bets_uc = Arc::new(MockBetsUc::default());
    let inventory_uc = Arc::new(MockInventoryUc::default());
    let social_uc = Arc::new(MockSocialUc::default());
    let taunts_uc = Arc::new(MockTauntsUc::default());
    let bot_config_repo = Arc::new(MockBotConfig::default());

    let svc = ResolveBettingBatchService::new(
        combat_repo.clone(),
        player_repo.clone(),
        wallet_repo.clone(),
        bets_uc.clone(),
        inventory_uc,
        social_uc,
        taunts_uc.clone(),
        bot_config_repo,
    );
    (svc, combat_repo, player_repo, wallet_repo, bets_uc, taunts_uc)
}

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn resolve_batch_empty_when_nothing_due() {
    let (svc, _, _, _, _, _) = build_service();
    let out = svc.resolve_batch().await.unwrap();
    assert!(out.is_empty());
}

#[tokio::test]
async fn resolve_batch_processes_due_combats() {
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service();

    let combat = make_combat("atk", "def", 100);
    combat_repo.due.lock().unwrap().push(combat);
    player_repo.insert(make_player("atk", 10, 100));
    player_repo.insert(make_player("def", 10, 100));
    wallet_repo.set_balance("g", "atk", 10_000);
    wallet_repo.set_balance("g", "def", 10_000);

    let out = svc.resolve_batch().await.unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(combat_repo.resolve_calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn resolve_batch_processes_stuck_combats() {
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service();

    combat_repo.stuck.lock().unwrap().push(make_combat("a", "b", 50));
    player_repo.insert(make_player("a", 5, 100));
    player_repo.insert(make_player("b", 5, 100));
    wallet_repo.set_balance("g", "a", 1000);
    wallet_repo.set_balance("g", "b", 1000);

    let out = svc.resolve_batch().await.unwrap();
    assert_eq!(out.len(), 1);
}

#[tokio::test]
async fn resolve_batch_merges_due_and_stuck() {
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service();

    combat_repo.due.lock().unwrap().push(make_combat("a1", "b1", 50));
    combat_repo.stuck.lock().unwrap().push(make_combat("a2", "b2", 50));
    for (a, b) in [("a1", "b1"), ("a2", "b2")] {
        player_repo.insert(make_player(a, 5, 100));
        player_repo.insert(make_player(b, 5, 100));
        wallet_repo.set_balance("g", a, 1000);
        wallet_repo.set_balance("g", b, 1000);
    }

    let out = svc.resolve_batch().await.unwrap();
    assert_eq!(out.len(), 2);
}

#[tokio::test]
async fn resolve_batch_skips_combat_when_player_missing() {
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service();

    // Combat present mais attacker absent du repo → resolve_one retourne NotFound
    // et le batch logge + continue.
    combat_repo.due.lock().unwrap().push(make_combat("ghost", "def", 100));
    // Pas d'insertion pour ghost
    player_repo.insert(make_player("def", 5, 100));
    wallet_repo.set_balance("g", "ghost", 1000);
    wallet_repo.set_balance("g", "def", 1000);

    let out = svc.resolve_batch().await.unwrap();
    assert_eq!(out.len(), 0, "combat avec player manquant doit etre skippe");
}

#[tokio::test]
async fn resolve_batch_updates_hp_for_both_players() {
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service();

    combat_repo.due.lock().unwrap().push(make_combat("atk", "def", 100));
    player_repo.insert(make_player("atk", 10, 100));
    player_repo.insert(make_player("def", 10, 100));
    wallet_repo.set_balance("g", "atk", 10_000);
    wallet_repo.set_balance("g", "def", 10_000);

    svc.resolve_batch().await.unwrap();

    let hp_calls = player_repo.update_hp_calls.lock().unwrap();
    assert!(hp_calls.iter().any(|(_, u, _, _)| u == "atk"));
    assert!(hp_calls.iter().any(|(_, u, _, _)| u == "def"));
}

#[tokio::test]
async fn resolve_batch_invokes_bets_resolve() {
    let (svc, combat_repo, player_repo, wallet_repo, bets_uc, _) = build_service();

    let combat = make_combat("atk", "def", 100);
    let combat_id = combat.id;
    combat_repo.due.lock().unwrap().push(combat);
    player_repo.insert(make_player("atk", 10, 100));
    player_repo.insert(make_player("def", 10, 100));
    wallet_repo.set_balance("g", "atk", 10_000);
    wallet_repo.set_balance("g", "def", 10_000);

    svc.resolve_batch().await.unwrap();

    let bets_calls = bets_uc.resolve_calls.lock().unwrap();
    assert_eq!(bets_calls.len(), 1);
    assert_eq!(bets_calls[0].0, combat_id);
}

#[tokio::test]
async fn resolve_batch_output_preserves_combat_metadata() {
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service();

    let combat = make_combat("atk", "def", 100);
    let combat_id = combat.id;
    combat_repo.due.lock().unwrap().push(combat);
    player_repo.insert(make_player("atk", 10, 100));
    player_repo.insert(make_player("def", 10, 100));
    wallet_repo.set_balance("g", "atk", 10_000);
    wallet_repo.set_balance("g", "def", 10_000);

    let out = svc.resolve_batch().await.unwrap();
    assert_eq!(out[0].combat_id, combat_id.to_string());
    assert_eq!(out[0].guild_id, "g");
    assert_eq!(out[0].channel_id.as_deref(), Some("c1"));
    assert_eq!(out[0].message_id.as_deref(), Some("msg1"));
}

// ══════════════════════════════════════════════════════════════════════
// Tests elargis : branches alternatives
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn resolve_batch_with_active_insurance_on_loser() {
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service_cfg(|cfg| {
        *cfg.inventory.active_insurance.lock().unwrap() = Some(Insurance {
            id: Uuid::new_v4(),
            is_scam: false,
            expires_at: Utc::now() + chrono::Duration::hours(1),
        });
    });
    combat_repo.due.lock().unwrap().push(make_combat("atk", "def", 500));
    player_repo.insert(make_player("atk", 10, 100));
    player_repo.insert(make_player("def", 10, 100));
    wallet_repo.set_balance("g", "atk", 10_000);
    wallet_repo.set_balance("g", "def", 10_000);

    // Doit reussir (pas de panic) : le path d'insurance est parcouru.
    svc.resolve_batch().await.unwrap();
}

#[tokio::test]
async fn resolve_batch_with_primes_to_claim() {
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service_cfg(|cfg| {
        *cfg.inventory.primes_amount.lock().unwrap() = 250;
    });
    combat_repo.due.lock().unwrap().push(make_combat("atk", "def", 100));
    player_repo.insert(make_player("atk", 10, 100));
    player_repo.insert(make_player("def", 10, 100));
    wallet_repo.set_balance("g", "atk", 5_000);
    wallet_repo.set_balance("g", "def", 5_000);
    svc.resolve_batch().await.unwrap();
    // Au moins un credit pour les primes devrait apparaitre (si qq a gagne).
}

#[tokio::test]
async fn resolve_batch_explosion_path_defender_special() {
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service();
    let mut combat = make_combat("atk", "def", 200);
    combat.special_attack = Some("surprise".into());
    combat.defender_special = Some("explosion".into());
    combat_repo.due.lock().unwrap().push(combat);
    player_repo.insert(make_player("atk", 10, 100));
    player_repo.insert(make_player("def", 10, 100));
    wallet_repo.set_balance("g", "atk", 5_000);
    wallet_repo.set_balance("g", "def", 5_000);
    // Pas de panic, engine gere cette combinaison.
    svc.resolve_batch().await.unwrap();
}

#[tokio::test]
async fn resolve_batch_giant_killer_underdog_scenario() {
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service();
    combat_repo.due.lock().unwrap().push(make_combat("weakling", "boss", 100));
    // weakling niveau 1, boss niveau 20 : giant-killer bonus possible
    player_repo.insert(make_player("weakling", 1, 100));
    player_repo.insert(make_player("boss", 20, 100));
    wallet_repo.set_balance("g", "weakling", 5_000);
    wallet_repo.set_balance("g", "boss", 50_000);
    svc.resolve_batch().await.unwrap();
    // Peu importe le resultat, le path giant_killer est exercise.
}

#[tokio::test]
async fn resolve_batch_with_active_server_events() {
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service_cfg(|cfg| {
        cfg.social.active_events.lock().unwrap().push(Event {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            event_type: "chaos".into(),
            active: true,
            expires_at: Utc::now() + chrono::Duration::hours(1),
            created_at: Utc::now(),
        });
    });
    combat_repo.due.lock().unwrap().push(make_combat("atk", "def", 100));
    player_repo.insert(make_player("atk", 10, 100));
    player_repo.insert(make_player("def", 10, 100));
    wallet_repo.set_balance("g", "atk", 5_000);
    wallet_repo.set_balance("g", "def", 5_000);
    svc.resolve_batch().await.unwrap();
}

#[tokio::test]
async fn resolve_batch_handles_multiple_combats_in_one_pass() {
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service();
    // 5 combats d'un coup
    for i in 0..5 {
        let a = format!("a{i}");
        let d = format!("d{i}");
        combat_repo.due.lock().unwrap().push(make_combat(&a, &d, 100));
        player_repo.insert(make_player(&a, 5, 100));
        player_repo.insert(make_player(&d, 5, 100));
        wallet_repo.set_balance("g", &a, 5_000);
        wallet_repo.set_balance("g", &d, 5_000);
    }
    let out = svc.resolve_batch().await.unwrap();
    assert_eq!(out.len(), 5);
}

#[tokio::test]
async fn resolve_batch_with_low_hp_defender() {
    // Defender a 1 HP → risque de mourir au premier round → impacte path coins_lost
    let (svc, combat_repo, player_repo, wallet_repo, _, _) = build_service();
    combat_repo.due.lock().unwrap().push(make_combat("atk", "def", 500));
    player_repo.insert(make_player("atk", 10, 100));
    player_repo.insert(make_player("def", 10, 1));
    wallet_repo.set_balance("g", "atk", 10_000);
    wallet_repo.set_balance("g", "def", 10_000);
    svc.resolve_batch().await.unwrap();
}

// Builder avec config custom des mocks inventory/social
struct BatchCfg {
    inventory: Arc<MockInventoryUc>,
    social: Arc<MockSocialUc>,
}

fn build_service_cfg(
    setup: impl FnOnce(&mut BatchCfg),
) -> (
    ResolveBettingBatchService,
    Arc<MockCombatRepo>,
    Arc<MockPlayerRepo>,
    Arc<MockWalletRepo>,
    Arc<MockBetsUc>,
    Arc<MockTauntsUc>,
) {
    let combat_repo = Arc::new(MockCombatRepo::default());
    let player_repo = Arc::new(MockPlayerRepo::default());
    let wallet_repo = Arc::new(MockWalletRepo::default());
    let bets_uc = Arc::new(MockBetsUc::default());
    let inventory_uc = Arc::new(MockInventoryUc::default());
    let social_uc = Arc::new(MockSocialUc::default());
    let taunts_uc = Arc::new(MockTauntsUc::default());
    let bot_config_repo = Arc::new(MockBotConfig::default());

    let mut cfg = BatchCfg {
        inventory: inventory_uc.clone(),
        social: social_uc.clone(),
    };
    setup(&mut cfg);

    let svc = ResolveBettingBatchService::new(
        combat_repo.clone(),
        player_repo.clone(),
        wallet_repo.clone(),
        bets_uc.clone(),
        inventory_uc,
        social_uc,
        taunts_uc.clone(),
        bot_config_repo,
    );
    (svc, combat_repo, player_repo, wallet_repo, bets_uc, taunts_uc)
}
