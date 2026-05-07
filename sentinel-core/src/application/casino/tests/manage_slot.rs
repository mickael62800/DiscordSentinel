//! Tests du service slot machine.
//!
//! Couvre les chemins de validation (mise hors borne, cooldown, daily bonus
//! deja claim, daily desactive) qui s executent AVANT toute tx DB. Les
//! tx mutations (debit/credit/log_spin) sont mockees `unimplemented!()` —
//! couvertes par les tests d integration Postgres separes.

use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::Postgres;
use sqlx::Transaction;
use crate::application::casino::manage_slot_service::ManageSlotService;
use crate::domain::entities::system::bot_config::BotDefinition;
use crate::domain::entities::system::bot_config::BotGuildConfig;
use crate::domain::entities::casino::slot::SlotJackpotPool;
use crate::domain::entities::casino::slot::SlotSpin;
use crate::domain::entities::casino::slot::SlotTopWinner;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_slot::ManageSlotUseCase;
use crate::ports::inbound::casino::manage_slot::SpinCommand;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::casino::manage_wallet::TxWalletMutation;
use crate::ports::inbound::casino::manage_wallet::WalletMutation;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::casino::slot_repository::SlotRepository;
// ── Mocks ──

#[derive(Default)]
struct MockSlotRepo {
    last_spin: StdMutex<Option<DateTime<Utc>>>,
    has_claimed: StdMutex<bool>,
    pool: StdMutex<Option<SlotJackpotPool>>,
    recent_returns: StdMutex<Vec<SlotSpin>>,
    top_returns: StdMutex<Vec<SlotTopWinner>>,
}

#[async_trait]
impl SlotRepository for MockSlotRepo {
    async fn get_jackpot_pool(&self, _g: &str) -> Result<Option<SlotJackpotPool>, DomainError> {
        Ok(self.pool.lock().unwrap().clone())
    }
    async fn init_jackpot_pool_if_absent(&self, _g: &str, _s: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn add_to_jackpot_pool_in_tx(
        &self, _: &mut dyn crate::ports::uow::DbTx, _: &str, _: i64, _: i64,
    ) -> Result<i64, DomainError> { unimplemented!() }
    async fn claim_jackpot_pool_in_tx(
        &self, _: &mut dyn crate::ports::uow::DbTx, _: &str, _: &str, _: i64, _: i64,
    ) -> Result<(), DomainError> { unimplemented!() }
    async fn log_spin_in_tx(
        &self, _: &mut dyn crate::ports::uow::DbTx, _: &SlotSpin,
    ) -> Result<(), DomainError> { unimplemented!() }
    async fn last_spin_at(&self, _: &str, _: &str) -> Result<Option<DateTime<Utc>>, DomainError> {
        Ok(*self.last_spin.lock().unwrap())
    }
    async fn has_claimed_daily_today(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(*self.has_claimed.lock().unwrap())
    }
    async fn mark_daily_claimed_in_tx(
        &self, _: &mut dyn crate::ports::uow::DbTx, _: &str, _: &str,
    ) -> Result<(), DomainError> { unimplemented!() }
    async fn recent_spins(&self, _: &str, _: i64) -> Result<Vec<SlotSpin>, DomainError> {
        Ok(self.recent_returns.lock().unwrap().clone())
    }
    async fn top_winners(
        &self, _: &str, _: i64, _: i64,
    ) -> Result<Vec<SlotTopWinner>, DomainError> {
        Ok(self.top_returns.lock().unwrap().clone())
    }
}

#[derive(Default)]
struct MockBotConfigRepo {
    entries: StdMutex<Vec<BotGuildConfig>>,
}

#[async_trait]
impl BotConfigRepository for MockBotConfigRepo {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> { Ok(vec![]) }
    async fn get_config(&self, _g: &str, _b: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(self.entries.lock().unwrap().clone())
    }
    async fn get_all_config(&self, _g: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(self.entries.lock().unwrap().clone())
    }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
}

struct MockWalletUc;

#[async_trait]
impl ManageWalletUseCase for MockWalletUc {
    async fn credit(&self, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<WalletMutation, DomainError> { unimplemented!() }
    async fn debit(&self, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<WalletMutation, DomainError> { unimplemented!() }
    async fn transfer(&self, _: &str, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<Vec<TauntEvent>, DomainError> { unimplemented!() }
    async fn get_balance(&self, _: &str, _: &str) -> Result<i64, DomainError> { Ok(0) }
    async fn credit_tx(&self, _: &mut dyn crate::ports::uow::DbTx, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<TxWalletMutation, DomainError> { unimplemented!() }
    async fn debit_tx(&self, _: &mut dyn crate::ports::uow::DbTx, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<TxWalletMutation, DomainError> { unimplemented!() }
    async fn post_commit_taunts(&self, _: &str, _: &str, _: &TxWalletMutation) -> Vec<TauntEvent> { vec![] }
}

// Pool postgres factice : le service en a besoin pour begin(). Comme nos
// tests court-circuitent avant begin(), un pool jamais utilise convient :
// on cree avec PgPoolOptions::connect_lazy qui ne tente pas de se connecter.
fn lazy_pool() -> sqlx::PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://nobody@localhost:5432/none")
        .expect("lazy pool")
}

fn make_service(
    slot_repo: Arc<MockSlotRepo>,
    bot_config: Arc<MockBotConfigRepo>,
) -> ManageSlotService {
    ManageSlotService::new(slot_repo, bot_config, Arc::new(MockWalletUc), lazy_pool())
}

fn entry(key: &str, value: &str) -> BotGuildConfig {
    BotGuildConfig {
        id: uuid::Uuid::nil(),
        guild_id: "g".into(),
        bot_name: "slot-bot".into(),
        config_key: key.into(),
        config_value: value.into(),
        updated_at: Utc::now(),
    }
}

fn cmd(mise: i64) -> SpinCommand {
    SpinCommand {
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "Alice".into(),
        mise,
        is_daily: false,
    }
}

// ══════════════════════════════════════════════════════════
// Validation : mise hors borne
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn spin_rejects_mise_below_min() {
    let svc = make_service(Arc::new(MockSlotRepo::default()), Arc::new(MockBotConfigRepo::default()));
    let err = svc.spin(cmd(5)).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(m) if m.contains("hors borne")));
}

#[tokio::test]
async fn spin_rejects_mise_above_max() {
    let svc = make_service(Arc::new(MockSlotRepo::default()), Arc::new(MockBotConfigRepo::default()));
    let err = svc.spin(cmd(99_999_999)).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(m) if m.contains("hors borne")));
}

#[tokio::test]
async fn spin_accepts_mise_exactly_at_min() {
    // mise = 10 (min defaut), on s arrete au moment de begin tx (lazy pool fail).
    // L erreur de validation NE doit pas etre levee.
    let svc = make_service(Arc::new(MockSlotRepo::default()), Arc::new(MockBotConfigRepo::default()));
    let err = svc.spin(cmd(10)).await.unwrap_err();
    // L erreur doit etre une erreur Internal (begin tx lazy fail), pas validation.
    assert!(!matches!(err, DomainError::ValidationError(m) if m.contains("hors borne")));
}

#[tokio::test]
async fn spin_accepts_mise_exactly_at_max() {
    let svc = make_service(Arc::new(MockSlotRepo::default()), Arc::new(MockBotConfigRepo::default()));
    let err = svc.spin(cmd(1000)).await.unwrap_err();
    assert!(!matches!(err, DomainError::ValidationError(m) if m.contains("hors borne")));
}

// ══════════════════════════════════════════════════════════
// Validation : cooldown
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn spin_rejects_when_within_cooldown() {
    let repo = MockSlotRepo::default();
    *repo.last_spin.lock().unwrap() = Some(Utc::now() - chrono::Duration::seconds(2));
    let svc = make_service(Arc::new(repo), Arc::new(MockBotConfigRepo::default()));
    // cooldown defaut = 5s, last_spin = il y a 2s -> doit refuser
    let err = svc.spin(cmd(50)).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(m) if m.contains("Cooldown")));
}

#[tokio::test]
async fn spin_passes_cooldown_when_enough_time_elapsed() {
    let repo = MockSlotRepo::default();
    *repo.last_spin.lock().unwrap() = Some(Utc::now() - chrono::Duration::seconds(60));
    let svc = make_service(Arc::new(repo), Arc::new(MockBotConfigRepo::default()));
    // 60s >> cooldown 5s : passe la validation cooldown (echoue plus tard sur la tx)
    let err = svc.spin(cmd(50)).await.unwrap_err();
    assert!(!matches!(err, DomainError::ValidationError(m) if m.contains("Cooldown")));
}

#[tokio::test]
async fn spin_with_zero_cooldown_never_blocks() {
    let repo = MockSlotRepo::default();
    *repo.last_spin.lock().unwrap() = Some(Utc::now());
    let bot_cfg = MockBotConfigRepo::default();
    *bot_cfg.entries.lock().unwrap() = vec![entry("cooldown_secs", "0")];
    let svc = make_service(Arc::new(repo), Arc::new(bot_cfg));
    let err = svc.spin(cmd(50)).await.unwrap_err();
    assert!(!matches!(err, DomainError::ValidationError(m) if m.contains("Cooldown")));
}

// ══════════════════════════════════════════════════════════
// Daily bonus
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn daily_rejects_when_disabled() {
    let bot_cfg = MockBotConfigRepo::default();
    *bot_cfg.entries.lock().unwrap() = vec![entry("daily_bonus_enabled", "false")];
    let svc = make_service(Arc::new(MockSlotRepo::default()), Arc::new(bot_cfg));
    let err = svc.claim_daily_bonus(cmd(0)).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(m) if m.contains("desactive")));
}

#[tokio::test]
async fn daily_rejects_when_already_claimed_today() {
    let repo = MockSlotRepo::default();
    *repo.has_claimed.lock().unwrap() = true;
    let svc = make_service(Arc::new(repo), Arc::new(MockBotConfigRepo::default()));
    let err = svc.claim_daily_bonus(cmd(0)).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(m) if m.contains("deja reclame")));
}

#[tokio::test]
async fn daily_passes_when_enabled_and_not_claimed() {
    let svc = make_service(Arc::new(MockSlotRepo::default()), Arc::new(MockBotConfigRepo::default()));
    let err = svc.claim_daily_bonus(cmd(0)).await.unwrap_err();
    // pas une erreur de validation business : echoue plus tard sur la tx
    assert!(!matches!(err, DomainError::ValidationError(m) if m.contains("desactive") || m.contains("deja")));
}

// ══════════════════════════════════════════════════════════
// load_config (parsing CSV + defaults)
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn config_parses_custom_min_max_bet() {
    let bot_cfg = MockBotConfigRepo::default();
    *bot_cfg.entries.lock().unwrap() = vec![
        entry("min_bet", "100"),
        entry("max_bet", "10000"),
    ];
    let svc = make_service(Arc::new(MockSlotRepo::default()), Arc::new(bot_cfg));

    // mise = 50 => en dessous du min custom 100 => rejet
    let err = svc.spin(cmd(50)).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(m) if m.contains("hors borne") && m.contains("100")));

    // mise = 5000 => OK (pas de validation error)
    let err2 = svc.spin(cmd(5000)).await.unwrap_err();
    assert!(!matches!(err2, DomainError::ValidationError(m) if m.contains("hors borne")));
}

#[tokio::test]
async fn config_invalid_symbols_returns_validation_error() {
    let bot_cfg = MockBotConfigRepo::default();
    // Une seule entree -> symbols.len() = 1 < 2 -> EmptySymbols
    *bot_cfg.entries.lock().unwrap() = vec![
        entry("symbols", "🍒"),
        entry("weights", "1"),
        entry("payout_3x_multipliers", "2"),
    ];
    let svc = make_service(Arc::new(MockSlotRepo::default()), Arc::new(bot_cfg));
    let err = svc.spin(cmd(50)).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(m) if m.contains("invalide")));
}

#[tokio::test]
async fn config_lengths_mismatch_returns_validation_error() {
    let bot_cfg = MockBotConfigRepo::default();
    *bot_cfg.entries.lock().unwrap() = vec![
        entry("symbols", "🍒,🍋,🍊"),
        entry("weights", "1,1"),                // mauvaise longueur
        entry("payout_3x_multipliers", "2,3,5"),
    ];
    let svc = make_service(Arc::new(MockSlotRepo::default()), Arc::new(bot_cfg));
    let err = svc.spin(cmd(50)).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(m) if m.contains("invalide")));
}

// ══════════════════════════════════════════════════════════
// Read-only delegations
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn get_jackpot_pool_zero_when_no_row() {
    let svc = make_service(Arc::new(MockSlotRepo::default()), Arc::new(MockBotConfigRepo::default()));
    assert_eq!(svc.get_jackpot_pool("g").await.unwrap(), 0);
}

#[tokio::test]
async fn get_jackpot_pool_returns_current_pool_when_row_exists() {
    let repo = MockSlotRepo::default();
    *repo.pool.lock().unwrap() = Some(SlotJackpotPool {
        guild_id: "g".into(),
        current_pool: 12345,
        last_won_by: None,
        last_won_at: None,
        last_won_amount: None,
    });
    let svc = make_service(Arc::new(repo), Arc::new(MockBotConfigRepo::default()));
    assert_eq!(svc.get_jackpot_pool("g").await.unwrap(), 12345);
}

#[tokio::test]
async fn recent_spins_delegates_to_repo() {
    let repo = MockSlotRepo::default();
    let now = Utc::now();
    *repo.recent_returns.lock().unwrap() = vec![SlotSpin {
        id: uuid::Uuid::nil(),
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "Alice".into(),
        mise: 50,
        symbols: vec!["🍒".into(), "🍒".into(), "🍋".into()],
        payout: 50,
        multiplier: 1.0,
        is_jackpot: false,
        is_free: false,
        created_at: now,
    }];
    let svc = make_service(Arc::new(repo), Arc::new(MockBotConfigRepo::default()));
    let out = svc.recent_spins("g", 10).await.unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].mise, 50);
    assert_eq!(out[0].payout, 50);
}

#[tokio::test]
async fn top_winners_delegates_to_repo() {
    let repo = MockSlotRepo::default();
    *repo.top_returns.lock().unwrap() = vec![SlotTopWinner {
        user_id: "u1".into(),
        username: "Alice".into(),
        total_payout: 5000,
        jackpot_count: 1,
        spin_count: 25,
    }];
    let svc = make_service(Arc::new(repo), Arc::new(MockBotConfigRepo::default()));
    let out = svc.top_winners("g", 7, 10).await.unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].total_payout, 5000);
    assert_eq!(out[0].jackpot_count, 1);
}
