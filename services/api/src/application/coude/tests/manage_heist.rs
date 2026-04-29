//! Tests de ManageCoudeHeistService. Couvre :
//! - get_cooldown_status (jamais braqué, cooldown actif, cooldown ecoule)
//! - get_prison_status (pas en prison, en prison, libere)
//! - attempt_heist early errors (prison, cooldown, caisse vide)
//! - attempt_heist success path (withdraw + credit + record)
//! - attempt_heist failure path (prison 24h)

use std::sync::Arc;
use std::sync::Mutex;
use async_trait::async_trait;
use chrono::Duration as ChronoDuration;
use chrono::Utc;
use uuid::Uuid;

use crate::application::coude::manage_heist_service::ManageCoudeHeistService;
use crate::domain::entities::system::bot_config::BotDefinition;
use crate::domain::entities::system::bot_config::BotGuildConfig;
use crate::domain::entities::coude::cashbox::CashboxRedistribution;
use crate::domain::entities::coude::cashbox::CashboxRedistributionEntry;
use crate::domain::entities::coude::cashbox::CashboxSource;
use crate::domain::entities::coude::cashbox::Cashbox;
use crate::domain::entities::coude::heist::HeistAttempt;
use crate::domain::entities::coude::inventory::CoudeInsurance;
use crate::domain::entities::coude::inventory::CoudeInventoryItem;
use crate::domain::entities::coude::inventory::CoudePrime;
use crate::domain::entities::coude::heist::PrisonState;
use crate::domain::entities::coude::inventory::NewCoudePrime;
use crate::domain::entities::casino::wallet::Wallet;
use crate::domain::entities::casino::wallet::WalletTransaction;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_heist::ManageCoudeHeistUseCase;
use crate::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::coude::cashbox_repository::CashboxRepository;
use crate::ports::outbound::coude::heist_repository::HeistRepository;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
// ── MockHeistRepo ──

#[derive(Default)]
struct MockHeistRepo {
    last_attempt: Mutex<Option<HeistAttempt>>,
    prison: Mutex<Option<PrisonState>>,
    record_calls: Mutex<Vec<(String, String, bool, i64, i32)>>,
    prison_calls: Mutex<Vec<(String, String, String)>>,
}

#[async_trait]
impl HeistRepository for MockHeistRepo {
    async fn last_attempt(&self, _: &str, _: &str) -> Result<Option<HeistAttempt>, DomainError> {
        Ok(self.last_attempt.lock().unwrap().clone())
    }
    async fn record_attempt(
        &self, g: &str, u: &str, success: bool, amount_stolen: i64, chance: i32, _: &[String],
    ) -> Result<HeistAttempt, DomainError> {
        self.record_calls.lock().unwrap().push((g.into(), u.into(), success, amount_stolen, chance));
        Ok(HeistAttempt {
            id: Uuid::new_v4(),
            guild_id: g.into(), user_id: u.into(),
            success, amount_stolen, chance_percent: chance,
            tools_used: vec![],
            attempted_at: Utc::now(),
        })
    }
    async fn get_prison(&self, _: &str, _: &str) -> Result<Option<PrisonState>, DomainError> {
        Ok(self.prison.lock().unwrap().clone())
    }
    async fn send_to_prison(
        &self, g: &str, u: &str, _released: chrono::DateTime<Utc>, reason: &str,
    ) -> Result<(), DomainError> {
        self.prison_calls.lock().unwrap().push((g.into(), u.into(), reason.into()));
        Ok(())
    }
}

// ── MockCashboxRepo ──

struct MockCashboxRepo {
    balance: Mutex<i64>,
    withdraw_calls: Mutex<Vec<(String, i64)>>,
}

impl Default for MockCashboxRepo {
    fn default() -> Self {
        Self { balance: Mutex::new(1_000_000), withdraw_calls: Mutex::new(vec![]) }
    }
}

#[async_trait]
impl CashboxRepository for MockCashboxRepo {
    async fn get_or_create(&self, g: &str) -> Result<Cashbox, DomainError> {
        Ok(Cashbox {
            guild_id: g.into(),
            balance: *self.balance.lock().unwrap(),
            total_collected: 0, total_redistributed: 0,
            last_redistribution_at: None,
            created_at: Utc::now(), updated_at: Utc::now(),
        })
    }
    async fn deposit(&self, _: &str, _: i64, _: CashboxSource) -> Result<(), DomainError> { Ok(()) }
    async fn claim_all_for_redistribution(&self, _: &str) -> Result<i64, DomainError> { Ok(0) }
    async fn withdraw(&self, g: &str, amount: i64) -> Result<i64, DomainError> {
        self.withdraw_calls.lock().unwrap().push((g.into(), amount));
        let mut b = self.balance.lock().unwrap();
        let taken = (*b).min(amount);
        *b -= taken;
        Ok(taken)
    }
    async fn record_redistribution(&self, _: &str, _: i64, _: Vec<(String, String, i64)>) -> Result<Uuid, DomainError> { Ok(Uuid::new_v4()) }
    async fn list_redistributions(&self, _: &str, _: i64) -> Result<Vec<CashboxRedistribution>, DomainError> { Ok(vec![]) }
    async fn list_entries(&self, _: Uuid) -> Result<Vec<CashboxRedistributionEntry>, DomainError> { Ok(vec![]) }
    async fn list_active_players(&self, _: &str, _: i64) -> Result<Vec<(String, String)>, DomainError> { Ok(vec![]) }
    async fn list_guilds_due_for_redistribution(&self, _: i64) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
}

// ── MockInventoryUc ──

#[derive(Default)]
struct MockInventoryUc {
    inventory: Mutex<Vec<CoudeInventoryItem>>,
    use_calls: Mutex<Vec<(String, String, String)>>,
}

#[async_trait]
impl ManageCoudeInventoryUseCase for MockInventoryUc {
    async fn list_inventory(&self, _: &str, _: &str) -> Result<Vec<CoudeInventoryItem>, DomainError> {
        Ok(self.inventory.lock().unwrap().clone())
    }
    async fn add_item(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn use_item(&self, g: &str, u: &str, key: &str) -> Result<bool, DomainError> {
        self.use_calls.lock().unwrap().push((g.into(), u.into(), key.into()));
        Ok(true)
    }
    async fn has_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn create_prime(&self, _: NewCoudePrime) -> Result<CoudePrime, DomainError> { unimplemented!() }
    async fn list_active_primes(&self, _: &str, _: &str) -> Result<Vec<CoudePrime>, DomainError> { Ok(vec![]) }
    async fn claim_primes(&self, _: &str, _: &str, _: &str, _: &str) -> Result<i64, DomainError> { Ok(0) }
    async fn buy_insurance(&self, _: &str, _: &str, _: bool, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn get_active_insurance(&self, _: &str, _: &str) -> Result<Option<CoudeInsurance>, DomainError> { Ok(None) }
    async fn expire_insurance(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
}

// ── MockWalletRepo ──

#[derive(Default)]
struct MockWalletRepo {
    credit_calls: Mutex<Vec<(String, String, i64, String)>>,
}

#[async_trait]
impl WalletRepository for MockWalletRepo {
    async fn get_or_create(&self, _: &str, _: &str, _: &str, _: i64) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn get(&self, _: &str, _: &str) -> Result<Option<Wallet>, DomainError> { Ok(None) }
    async fn credit(&self, g: &str, u: &str, amount: i64, source: &str, _: &str) -> Result<Wallet, DomainError> {
        self.credit_calls.lock().unwrap().push((g.into(), u.into(), amount, source.into()));
        Ok(Wallet {
            id: Uuid::new_v4(), guild_id: g.into(), user_id: u.into(), username: "x".into(),
            coins: amount, total_earned: amount, total_spent: 0,
            created_at: Utc::now(), updated_at: Utc::now(),
        })
    }
    async fn debit(&self, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn transfer(&self, _: &str, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn pay_combat_atomic(&self, _: &str, _: &str, _: i64, _: &str, _: i64, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn leaderboard(&self, _: &str, _: i64) -> Result<Vec<Wallet>, DomainError> { Ok(vec![]) }
    async fn get_transactions(&self, _: &str, _: &str, _: i64) -> Result<Vec<WalletTransaction>, DomainError> { Ok(vec![]) }
    async fn list_by_guild(&self, _: &str) -> Result<Vec<Wallet>, DomainError> { Ok(vec![]) }
    async fn reset_wallet(&self, _: &str, _: &str, _: i64) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn reset_all_wallets(&self, _: &str, _: i64) -> Result<u64, DomainError> { Ok(0) }
}

// ── MockBotConfig (pour load_balance) ──

#[derive(Default)]
struct MockBotConfig {
    rows: Mutex<Vec<BotGuildConfig>>,
}

#[async_trait]
impl BotConfigRepository for MockBotConfig {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> { Ok(vec![]) }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(self.rows.lock().unwrap().clone())
    }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> { Ok(vec![]) }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
}

// ── Helper builder ──

fn build_service(
    heist: Arc<MockHeistRepo>,
    cashbox: Arc<MockCashboxRepo>,
    inventory: Arc<MockInventoryUc>,
    wallet: Arc<MockWalletRepo>,
    bot_config: Arc<MockBotConfig>,
) -> ManageCoudeHeistService {
    ManageCoudeHeistService::new(heist, cashbox, inventory, wallet, bot_config)
}

fn default_service_parts() -> (
    Arc<MockHeistRepo>, Arc<MockCashboxRepo>, Arc<MockInventoryUc>,
    Arc<MockWalletRepo>, Arc<MockBotConfig>,
) {
    (
        Arc::new(MockHeistRepo::default()),
        Arc::new(MockCashboxRepo::default()),
        Arc::new(MockInventoryUc::default()),
        Arc::new(MockWalletRepo::default()),
        Arc::new(MockBotConfig::default()),
    )
}

// ═══════════════════════════════════════════════════════════════════
// get_cooldown_status
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn cooldown_ready_when_never_attempted() {
    let (h, c, i, w, b) = default_service_parts();
    let svc = build_service(h, c, i, w, b);
    let s = svc.get_cooldown_status("g", "u").await.unwrap();
    assert!(s.ready);
    assert!(s.next_attempt_at.is_none());
    assert!(s.last_success.is_none());
}

#[tokio::test]
async fn cooldown_not_ready_when_recent_attempt() {
    let (h, c, i, w, b) = default_service_parts();
    *h.last_attempt.lock().unwrap() = Some(HeistAttempt {
        id: Uuid::new_v4(),
        guild_id: "g".into(), user_id: "u".into(),
        success: true, amount_stolen: 1000, chance_percent: 50,
        tools_used: vec![],
        attempted_at: Utc::now(), // tout recent
    });
    let svc = build_service(h, c, i, w, b);
    let s = svc.get_cooldown_status("g", "u").await.unwrap();
    assert!(!s.ready);
    assert_eq!(s.last_success, Some(true));
    assert!(s.next_attempt_at.is_some());
}

#[tokio::test]
async fn cooldown_ready_when_old_attempt() {
    let (h, c, i, w, b) = default_service_parts();
    *h.last_attempt.lock().unwrap() = Some(HeistAttempt {
        id: Uuid::new_v4(),
        guild_id: "g".into(), user_id: "u".into(),
        success: false, amount_stolen: 0, chance_percent: 30,
        tools_used: vec![],
        attempted_at: Utc::now() - ChronoDuration::days(30), // tres vieux
    });
    let svc = build_service(h, c, i, w, b);
    let s = svc.get_cooldown_status("g", "u").await.unwrap();
    assert!(s.ready);
    assert_eq!(s.last_success, Some(false));
}

// ═══════════════════════════════════════════════════════════════════
// get_prison_status
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn prison_status_not_in_prison_by_default() {
    let (h, c, i, w, b) = default_service_parts();
    let svc = build_service(h, c, i, w, b);
    let s = svc.get_prison_status("g", "u").await.unwrap();
    assert!(!s.in_prison);
    assert!(s.released_at.is_none());
    assert!(s.reason.is_none());
}

#[tokio::test]
async fn prison_status_active_when_released_at_in_future() {
    let (h, c, i, w, b) = default_service_parts();
    *h.prison.lock().unwrap() = Some(PrisonState {
        guild_id: "g".into(), user_id: "u".into(),
        released_at: Utc::now() + ChronoDuration::hours(10),
        reason: "heist_failed".into(),
        created_at: Utc::now(),
    });
    let svc = build_service(h, c, i, w, b);
    let s = svc.get_prison_status("g", "u").await.unwrap();
    assert!(s.in_prison);
    assert_eq!(s.reason.as_deref(), Some("heist_failed"));
}

#[tokio::test]
async fn prison_status_released_when_released_at_past() {
    let (h, c, i, w, b) = default_service_parts();
    *h.prison.lock().unwrap() = Some(PrisonState {
        guild_id: "g".into(), user_id: "u".into(),
        released_at: Utc::now() - ChronoDuration::hours(1), // passe
        reason: "heist_failed".into(),
        created_at: Utc::now() - ChronoDuration::days(2),
    });
    let svc = build_service(h, c, i, w, b);
    let s = svc.get_prison_status("g", "u").await.unwrap();
    assert!(!s.in_prison);
    assert!(s.released_at.is_some()); // l'info est conservee
}

// ═══════════════════════════════════════════════════════════════════
// attempt_heist — early errors
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn attempt_heist_forbidden_when_in_prison() {
    let (h, c, i, w, b) = default_service_parts();
    *h.prison.lock().unwrap() = Some(PrisonState {
        guild_id: "g".into(), user_id: "u".into(),
        released_at: Utc::now() + ChronoDuration::hours(10),
        reason: "heist_failed".into(),
        created_at: Utc::now(),
    });
    let svc = build_service(h, c, i, w, b);
    let err = svc.attempt_heist("g", "u").await.unwrap_err();
    assert!(matches!(err, DomainError::Forbidden(_)));
    assert!(format!("{err:?}").contains("prison"));
}

#[tokio::test]
async fn attempt_heist_forbidden_when_cooldown_active() {
    let (h, c, i, w, b) = default_service_parts();
    *h.last_attempt.lock().unwrap() = Some(HeistAttempt {
        id: Uuid::new_v4(),
        guild_id: "g".into(), user_id: "u".into(),
        success: true, amount_stolen: 500, chance_percent: 50,
        tools_used: vec![],
        attempted_at: Utc::now(),
    });
    let svc = build_service(h, c, i, w, b);
    let err = svc.attempt_heist("g", "u").await.unwrap_err();
    assert!(matches!(err, DomainError::Forbidden(_)));
    assert!(format!("{err:?}").contains("Cooldown"));
}

#[tokio::test]
async fn attempt_heist_forbidden_when_cashbox_empty() {
    let (h, c, i, w, b) = default_service_parts();
    *c.balance.lock().unwrap() = 0;
    let svc = build_service(h, c, i, w, b);
    let err = svc.attempt_heist("g", "u").await.unwrap_err();
    assert!(matches!(err, DomainError::Forbidden(_)));
    assert!(format!("{err:?}").contains("caisse"));
}

// ═══════════════════════════════════════════════════════════════════
// attempt_heist — full paths (success / failure)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn attempt_heist_records_attempt_always() {
    // Meme si succes ou echec, record_attempt doit etre appele.
    let (h, c, i, w, b) = default_service_parts();
    let svc = build_service(h.clone(), c, i, w, b);
    let _ = svc.attempt_heist("g", "u").await.unwrap();
    let records = h.record_calls.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, "g");
    assert_eq!(records[0].1, "u");
}

#[tokio::test]
async fn attempt_heist_failure_sends_to_prison() {
    // Avec 0 outils → chance = base (~15%). Difficile de forcer
    // determinisme, mais on peut verifier la coherence : si outcome.success
    // est false, alors send_to_prison a ete appele.
    let (h, c, i, w, b) = default_service_parts();
    let svc = build_service(h.clone(), c, i, w, b);

    // On loop jusqu'a avoir un echec (chance de base faible → typiquement
    // 1-3 iterations). On reset entre chaque pour bypass le cooldown.
    let mut saw_failure = false;
    for _ in 0..50 {
        *h.last_attempt.lock().unwrap() = None;
        h.record_calls.lock().unwrap().clear();
        h.prison_calls.lock().unwrap().clear();
        let out = svc.attempt_heist("g", "u").await.unwrap();
        if !out.success {
            assert!(out.prison_released_at.is_some());
            assert_eq!(out.amount_stolen, 0);
            assert_eq!(h.prison_calls.lock().unwrap().len(), 1);
            saw_failure = true;
            break;
        }
    }
    assert!(saw_failure, "devrait voir au moins un echec en 50 iterations");
}

#[tokio::test]
async fn attempt_heist_success_withdraws_and_credits() {
    // Seed avec tous les outils HEIST_TOOLS -> chance maximale (55%).
    // Rend le test stable : P(aucun succes en 50 iterations) ~= 1e-18.
    let (h, c, i, w, b) = default_service_parts();
    use crate::domain::entities::coude::heist::HEIST_TOOLS;
    for tool in HEIST_TOOLS {
        i.inventory.lock().unwrap().push(CoudeInventoryItem {
            guild_id: "g".into(), user_id: "u".into(),
            item_key: tool.key.to_string(), quantity: 10,
        });
    }
    let svc = build_service(h.clone(), c.clone(), i, w.clone(), b);

    let mut saw_success = false;
    for _ in 0..50 {
        *h.last_attempt.lock().unwrap() = None;
        *c.balance.lock().unwrap() = 1_000_000;
        h.record_calls.lock().unwrap().clear();
        c.withdraw_calls.lock().unwrap().clear();
        w.credit_calls.lock().unwrap().clear();

        let out = svc.attempt_heist("g", "u").await.unwrap();
        if out.success {
            assert!(out.amount_stolen > 0);
            assert!(out.prison_released_at.is_none());
            assert_eq!(c.withdraw_calls.lock().unwrap().len(), 1);
            assert_eq!(w.credit_calls.lock().unwrap().len(), 1);
            let credit = &w.credit_calls.lock().unwrap()[0];
            assert_eq!(credit.0, "g");
            assert_eq!(credit.1, "u");
            assert_eq!(credit.3, "coude_heist_success");
            saw_success = true;
            break;
        }
    }
    assert!(saw_success, "devrait voir au moins un succes en 50 iterations avec tous les outils");
}

#[tokio::test]
async fn attempt_heist_filters_unknown_items_from_tools() {
    // Les items dans l'inventaire qui ne sont pas dans HEIST_TOOLS doivent
    // etre ignores (pas envoyes a compute_success_chance).
    let (h, c, i, w, b) = default_service_parts();
    i.inventory.lock().unwrap().extend(vec![
        CoudeInventoryItem {
            guild_id: "g".into(), user_id: "u".into(),
            item_key: "potion".into(), quantity: 5, // pas un outil braquage
        },
        CoudeInventoryItem {
            guild_id: "g".into(), user_id: "u".into(),
            item_key: "not_an_item".into(), quantity: 1,
        },
    ]);
    let svc = build_service(h, c, i.clone(), w, b);
    // Ne doit pas crash : les items inconnus sont filtres avant compute_chance
    let _ = svc.attempt_heist("g", "u").await.unwrap();
}

#[tokio::test]
async fn attempt_heist_skips_zero_quantity_items() {
    let (h, c, i, w, b) = default_service_parts();
    i.inventory.lock().unwrap().push(CoudeInventoryItem {
        guild_id: "g".into(), user_id: "u".into(),
        item_key: "potion".into(), quantity: 0,
    });
    let svc = build_service(h, c, i.clone(), w, b);
    let _ = svc.attempt_heist("g", "u").await.unwrap();
    // Aucun use_item appele car quantity = 0 → tool_keys vide
    assert!(i.use_calls.lock().unwrap().is_empty());
}
