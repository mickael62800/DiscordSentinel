//! Tests de ExpireCombatsBatchService. Couvre le flow complet : claim →
//! penalty debit → cashbox deposit → stats record → refund bets, avec les
//! chemins degrades (debit echoue → pas de deposit, pas de stats).

use std::sync::Arc;
use std::sync::Mutex;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::application::coude::combat::expire_batch::ExpireCombatsBatchService;
use sentinel_core::domain::entities::coude::cashbox::CashboxSource;
use sentinel_core::domain::entities::coude::combat::CombatResolution;
use sentinel_core::domain::entities::coude::bet::Bet;
use sentinel_core::domain::entities::coude::cashbox::Cashbox;
use sentinel_core::domain::entities::coude::combat::Combat;
use sentinel_core::domain::entities::coude::combat::NewCoudeCombat;
use sentinel_core::domain::entities::coude::bet::RefundSummary;
use sentinel_core::domain::entities::casino::wallet::Wallet;
use sentinel_core::domain::entities::casino::wallet::WalletTransaction;
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::coude::expire_combats_batch::ExpireCombatsBatchUseCase;
use crate::ports::inbound::coude::manage_bets::ManageCoudeBetsUseCase;
use crate::ports::inbound::coude::manage_bets::PlaceBetOutcome;
use crate::ports::inbound::coude::manage_bets::ResolveBetsOutcome;
use sentinel_core::domain::entities::coude::bet::NewCoudeBet;
use crate::ports::outbound::coude::cashbox_repository::CashboxRepository;
use crate::ports::outbound::coude::combat_repository::CombatRepository;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
// ── MockCombatRepo (seul claim_expired_pending_combats est exerce) ──

#[derive(Default)]
struct MockCombatRepo {
    expired: Mutex<Vec<Combat>>,
}

#[async_trait]
impl CombatRepository for MockCombatRepo {
    async fn list(&self, _: &str, _: Option<&str>, _: i64) -> Result<Vec<Combat>, DomainError> { Ok(vec![]) }
    async fn get(&self, _: Uuid) -> Result<Option<Combat>, DomainError> { Ok(None) }
    async fn get_pending_for_attacker(&self, _: &str, _: &str) -> Result<Option<Combat>, DomainError> { Ok(None) }
    async fn get_pending_for_defender(&self, _: &str, _: &str) -> Result<Option<Combat>, DomainError> { Ok(None) }
    async fn list_expired_pending(&self) -> Result<Vec<Combat>, DomainError> { Ok(vec![]) }
    async fn claim_due_betting_combats(&self, _: i64) -> Result<Vec<Combat>, DomainError> { Ok(vec![]) }
    async fn claim_stuck_resolving_combats(&self, _: i64) -> Result<Vec<Combat>, DomainError> { Ok(vec![]) }
    async fn claim_expired_pending_combats(&self, _: i64) -> Result<Vec<Combat>, DomainError> {
        Ok(self.expired.lock().unwrap().clone())
    }
    async fn get_betting_for_participant(&self, _: &str, _: &str) -> Result<Option<Combat>, DomainError> { Ok(None) }
    async fn create(&self, _: NewCoudeCombat) -> Result<Combat, DomainError> { unimplemented!() }
    async fn resolve(&self, _: Uuid, _: CombatResolution) -> Result<bool, DomainError> { Ok(true) }
    async fn set_betting(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn expire(&self, _: Uuid) -> Result<bool, DomainError> { Ok(true) }
    async fn cancel_pending(&self, _: Uuid) -> Result<bool, DomainError> { Ok(true) }
    async fn set_defender_special(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn mark_unresolved_bets_lost(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
}

// ── MockPlayerRepo (seules record_coins_lost + increment_cowardice sont exercees) ──

#[derive(Default)]
struct MockPlayerRepo {
    record_lost_calls: Mutex<Vec<(String, String, i64)>>,
    cowardice_calls: Mutex<Vec<(String, String)>>,
}

#[async_trait]
impl PlayerRepository for MockPlayerRepo {
    async fn get_or_create(&self, _: &str, _: &str, _: &str) -> Result<sentinel_core::domain::entities::coude::player::Player, DomainError> { unimplemented!() }
    async fn get(&self, _: &str, _: &str) -> Result<Option<sentinel_core::domain::entities::coude::player::Player>, DomainError> { Ok(None) }
    async fn list(&self, _: &str, _: i64) -> Result<Vec<sentinel_core::domain::entities::coude::player::Player>, DomainError> { Ok(vec![]) }
    async fn random_active(&self, _: &str, _: i64, _: i64) -> Result<Vec<sentinel_core::domain::entities::coude::player::Player>, DomainError> { Ok(vec![]) }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
    async fn update_class(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn add_xp(&self, _: &str, _: &str, _: i64) -> Result<Option<sentinel_core::domain::entities::coude::player::XpProgress>, DomainError> { Ok(None) }
    async fn spend_stat_point(&self, _: &str, _: &str, _: sentinel_core::domain::entities::coude::player::CombatStat) -> Result<Option<sentinel_core::domain::entities::coude::player::Player>, DomainError> { Ok(None) }
    async fn reset_stats(&self, _: &str, _: &str, _: i64) -> Result<Option<sentinel_core::domain::entities::coude::player::Player>, DomainError> { Ok(None) }
    async fn record_coins_earned(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn record_coins_lost(&self, g: &str, u: &str, amount: i64) -> Result<bool, DomainError> {
        self.record_lost_calls.lock().unwrap().push((g.into(), u.into(), amount));
        Ok(true)
    }
    async fn record_win(&self, _: &str, _: &str, _: i64, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn record_loss(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn record_draw(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn touch_win_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn touch_loss_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn reset_combat_streaks(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn touch_steal_victim_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn reset_steal_victim_streak(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn touch_bj_win_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn touch_bj_bust_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn reset_bj_bust_streak(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn increment_cowardice(&self, g: &str, u: &str) -> Result<Option<i32>, DomainError> {
        self.cowardice_calls.lock().unwrap().push((g.into(), u.into()));
        Ok(Some(1))
    }
    async fn increment_chaos(&self, _: &str, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn update_hp(&self, _: &str, _: &str, _: i32, _: i32) -> Result<(), DomainError> { Ok(()) }
    async fn full_heal(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn regen_hp_tick(&self, _: f64, _: f64, _: f64, _: f64) -> Result<u64, DomainError> { Ok(0) }
}

// ── MockWalletRepo ──

#[derive(Default)]
struct MockWalletRepo {
    debit_calls: Mutex<Vec<(String, String, i64)>>,
    debit_should_fail: Mutex<bool>,
}

#[async_trait]
impl WalletRepository for MockWalletRepo {
    async fn get_or_create(&self, _: &str, _: &str, _: &str, _: i64) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn get(&self, _: &str, _: &str) -> Result<Option<Wallet>, DomainError> { Ok(None) }
    async fn credit(&self, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn debit(&self, g: &str, u: &str, amount: i64, _: &str, _: &str) -> Result<Wallet, DomainError> {
        if *self.debit_should_fail.lock().unwrap() {
            return Err(DomainError::ValidationError("Solde insuffisant".into()));
        }
        self.debit_calls.lock().unwrap().push((g.into(), u.into(), amount));
        Ok(Wallet {
            id: Uuid::new_v4(), guild_id: g.into(), user_id: u.into(), username: "x".into(),
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

// ── MockCashboxRepo ──

#[derive(Default)]
struct MockCashboxRepo {
    deposit_calls: Mutex<Vec<(String, i64, CashboxSource)>>,
}

#[async_trait]
impl CashboxRepository for MockCashboxRepo {
    async fn get_or_create(&self, g: &str) -> Result<Cashbox, DomainError> {
        Ok(Cashbox {
            guild_id: g.into(), balance: 0, total_collected: 0, total_redistributed: 0,
            last_redistribution_at: None, created_at: Utc::now(), updated_at: Utc::now(),
        })
    }
    async fn deposit(&self, g: &str, amount: i64, source: CashboxSource) -> Result<(), DomainError> {
        self.deposit_calls.lock().unwrap().push((g.into(), amount, source));
        Ok(())
    }
    async fn claim_all_for_redistribution(&self, _: &str) -> Result<i64, DomainError> { Ok(0) }
    async fn withdraw(&self, _: &str, _: i64) -> Result<i64, DomainError> { Ok(0) }
    async fn record_redistribution(&self, _: &str, _: i64, _: Vec<(String, String, i64)>) -> Result<Uuid, DomainError> { Ok(Uuid::new_v4()) }
    async fn list_redistributions(&self, _: &str, _: i64) -> Result<Vec<sentinel_core::domain::entities::coude::cashbox::CashboxRedistribution>, DomainError> { Ok(vec![]) }
    async fn list_entries(&self, _: Uuid) -> Result<Vec<sentinel_core::domain::entities::coude::cashbox::CashboxRedistributionEntry>, DomainError> { Ok(vec![]) }
    async fn list_active_players(&self, _: &str, _: i64) -> Result<Vec<(String, String)>, DomainError> { Ok(vec![]) }
    async fn list_guilds_due_for_redistribution(&self, _: i64) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
}

// ── MockBetsUc ──

#[derive(Default)]
struct MockBetsUc {
    refund_calls: Mutex<Vec<Uuid>>,
}

#[async_trait]
impl ManageCoudeBetsUseCase for MockBetsUc {
    async fn place(&self, _: NewCoudeBet) -> Result<PlaceBetOutcome, DomainError> { unimplemented!() }
    async fn list_for_combat(&self, _: Uuid) -> Result<Vec<Bet>, DomainError> { Ok(vec![]) }
    async fn resolve(&self, _: Uuid, _: Option<String>) -> Result<ResolveBetsOutcome, DomainError> { unimplemented!() }
    async fn refund(&self, combat_id: Uuid) -> Result<RefundSummary, DomainError> {
        self.refund_calls.lock().unwrap().push(combat_id);
        Ok(RefundSummary { refunded_count: 0, refunded_total: 0 })
    }
}

// ── Helpers ──

fn make_combat(guild: &str, defender: &str, mise: i64) -> Combat {
    Combat {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        channel_id: Some("c1".into()),
        attacker_id: "attacker".into(),
        attacker_name: "Atk".into(),
        defender_id: defender.into(),
        defender_name: "Def".into(),
        mise,
        status: "expired".into(),
        winner_id: None,
        attacker_roll: None, defender_roll: None,
        chaos_event: None, special_attack: None, defender_special: None,
        coins_transferred: None, result_message: None, message_id: None,
        created_at: Utc::now(),
        accepted_at: None, resolved_at: None,
    }
}

fn build_service(
    combat: Arc<MockCombatRepo>,
    player: Arc<MockPlayerRepo>,
    wallet: Arc<MockWalletRepo>,
    cashbox: Arc<MockCashboxRepo>,
    bets: Arc<MockBetsUc>,
) -> ExpireCombatsBatchService {
    ExpireCombatsBatchService::new(combat, player, wallet, cashbox, bets)
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn expire_batch_empty_returns_no_outputs() {
    let svc = build_service(
        Arc::new(MockCombatRepo::default()),
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockWalletRepo::default()),
        Arc::new(MockCashboxRepo::default()),
        Arc::new(MockBetsUc::default()),
    );
    let out = svc.expire_batch().await.unwrap();
    assert!(out.is_empty());
}

#[tokio::test]
async fn expire_batch_happy_path_penalty_deposit_stats_refund() {
    let combat = Arc::new(MockCombatRepo::default());
    combat.expired.lock().unwrap().push(make_combat("g1", "lazy-def", 1000));
    let player = Arc::new(MockPlayerRepo::default());
    let wallet = Arc::new(MockWalletRepo::default());
    let cashbox = Arc::new(MockCashboxRepo::default());
    let bets = Arc::new(MockBetsUc::default());

    let svc = build_service(combat.clone(), player.clone(), wallet.clone(), cashbox.clone(), bets.clone());
    let out = svc.expire_batch().await.unwrap();

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].guild_id, "g1");
    assert_eq!(out[0].defender_id, "lazy-def");
    // cowardice_penalty(1000) = 20% = 200
    assert_eq!(out[0].penalty, 200);

    // Debit wallet 200 au defender
    let debits = wallet.debit_calls.lock().unwrap();
    assert_eq!(debits.len(), 1);
    assert_eq!(debits[0], ("g1".into(), "lazy-def".into(), 200));

    // Deposit cashbox 200 (CowardicePenalty)
    let deposits = cashbox.deposit_calls.lock().unwrap();
    assert_eq!(deposits.len(), 1);
    assert_eq!(deposits[0].0, "g1");
    assert_eq!(deposits[0].1, 200);
    assert_eq!(deposits[0].2, CashboxSource::CowardicePenalty);

    // record_coins_lost avec le meme montant
    let records = player.record_lost_calls.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0], ("g1".into(), "lazy-def".into(), 200));

    // increment_cowardice
    let cow = player.cowardice_calls.lock().unwrap();
    assert_eq!(cow.len(), 1);

    // refund des paris
    assert_eq!(bets.refund_calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn expire_batch_small_mise_penalty_clamped_to_minimum_one() {
    let combat = Arc::new(MockCombatRepo::default());
    combat.expired.lock().unwrap().push(make_combat("g", "d", 3)); // 20% of 3 = 0.6 → clamp 1
    let wallet = Arc::new(MockWalletRepo::default());
    let svc = build_service(
        combat,
        Arc::new(MockPlayerRepo::default()),
        wallet.clone(),
        Arc::new(MockCashboxRepo::default()),
        Arc::new(MockBetsUc::default()),
    );
    let out = svc.expire_batch().await.unwrap();
    assert_eq!(out[0].penalty, 1);
    let debits = wallet.debit_calls.lock().unwrap();
    assert_eq!(debits[0].2, 1);
}

#[tokio::test]
async fn expire_batch_debit_failure_skips_deposit_and_stats() {
    let combat = Arc::new(MockCombatRepo::default());
    combat.expired.lock().unwrap().push(make_combat("g", "broke", 500));
    let wallet = Arc::new(MockWalletRepo::default());
    *wallet.debit_should_fail.lock().unwrap() = true;
    let cashbox = Arc::new(MockCashboxRepo::default());
    let player = Arc::new(MockPlayerRepo::default());
    let bets = Arc::new(MockBetsUc::default());

    let svc = build_service(combat, player.clone(), wallet.clone(), cashbox.clone(), bets.clone());
    let out = svc.expire_batch().await.unwrap();

    // Output encore emis (le combat est quand meme expire)
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].penalty, 100); // 20% de 500

    // Pas de debit enregistre (il a echoue)
    assert!(wallet.debit_calls.lock().unwrap().is_empty());
    // Pas de deposit cashbox (debit_ok false)
    assert!(cashbox.deposit_calls.lock().unwrap().is_empty());
    // Pas de record_coins_lost (gated sur debit_ok)
    assert!(player.record_lost_calls.lock().unwrap().is_empty());
    // Mais cowardice incremented quand meme (toujours applique)
    assert_eq!(player.cowardice_calls.lock().unwrap().len(), 1);
    // Et refund toujours tente
    assert_eq!(bets.refund_calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn expire_batch_multiple_combats_processes_all() {
    let combat = Arc::new(MockCombatRepo::default());
    {
        let mut e = combat.expired.lock().unwrap();
        e.push(make_combat("g1", "d1", 100));
        e.push(make_combat("g1", "d2", 200));
        e.push(make_combat("g2", "d3", 500));
    }
    let wallet = Arc::new(MockWalletRepo::default());
    let cashbox = Arc::new(MockCashboxRepo::default());
    let bets = Arc::new(MockBetsUc::default());

    let svc = build_service(
        combat,
        Arc::new(MockPlayerRepo::default()),
        wallet.clone(),
        cashbox.clone(),
        bets.clone(),
    );
    let out = svc.expire_batch().await.unwrap();

    assert_eq!(out.len(), 3);
    assert_eq!(wallet.debit_calls.lock().unwrap().len(), 3);
    assert_eq!(cashbox.deposit_calls.lock().unwrap().len(), 3);
    assert_eq!(bets.refund_calls.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn expire_batch_output_preserves_channel_id() {
    let combat = Arc::new(MockCombatRepo::default());
    combat.expired.lock().unwrap().push(make_combat("g", "d", 100));
    let svc = build_service(
        combat,
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockWalletRepo::default()),
        Arc::new(MockCashboxRepo::default()),
        Arc::new(MockBetsUc::default()),
    );
    let out = svc.expire_batch().await.unwrap();
    assert_eq!(out[0].channel_id, "c1");
    assert_eq!(out[0].defender_name, "Def");
}

#[tokio::test]
async fn expire_batch_output_empty_channel_when_none() {
    let combat = Arc::new(MockCombatRepo::default());
    let mut c = make_combat("g", "d", 100);
    c.channel_id = None;
    combat.expired.lock().unwrap().push(c);
    let svc = build_service(
        combat,
        Arc::new(MockPlayerRepo::default()),
        Arc::new(MockWalletRepo::default()),
        Arc::new(MockCashboxRepo::default()),
        Arc::new(MockBetsUc::default()),
    );
    let out = svc.expire_batch().await.unwrap();
    assert_eq!(out[0].channel_id, "");
}
