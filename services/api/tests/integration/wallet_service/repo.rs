//! Tests d'integration pour ManageWalletService — couvre les modes
//! standalone (credit/debit/transfer/get_balance) et tx (credit_tx/debit_tx
//! + post_commit_taunts) avec un vrai PgPool.

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::casino::wallet_repository::PgWalletRepository;
use sentinel_api::application::casino::manage_wallet_service::ManageWalletService;
use sentinel_api::domain::entities::coude::taunt::CoudeTauntsConfig;
use sentinel_api::domain::entities::coude::taunt::TauntEvent;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use sentinel_api::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use sentinel_api::ports::inbound::casino::manage_wallet::TxWalletMutation;
use sentinel_api::ports::outbound::casino::wallet_repository::WalletRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}

fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

// Taunts stub : emet jackpot au-dessus de 10k, bankruptcy toujours.
struct TestTaunts;
#[async_trait]
impl ManageCoudeTauntsUseCase for TestTaunts {
    async fn on_player_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_lost(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_drew(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn on_player_stolen_from(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_defended_steal(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn on_bankruptcy(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(Some(TauntEvent {
            channel_id: "c".into(), target_user_id: "u".into(),
            message: "bankrupt".into(), nickname_suffix: "".into(),
            streak_kind: "bankruptcy", streak_value: 1,
        }))
    }
    async fn on_jackpot(&self, _: &str, _: &str, amount: i64) -> Result<Option<TauntEvent>, DomainError> {
        if amount >= 10_000 {
            Ok(Some(TauntEvent {
                channel_id: "c".into(), target_user_id: "u".into(),
                message: "jackpot".into(), nickname_suffix: "".into(),
                streak_kind: "jackpot", streak_value: 1,
            }))
        } else { Ok(None) }
    }
    async fn on_generous_donor(&self, _: &str, _: &str, _: i64) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_bj_natural(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_bj_hand_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_bj_hand_bust(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn get_config(&self, _: &str) -> Result<CoudeTauntsConfig, DomainError> { unimplemented!() }
    async fn set_channel(&self, _: &str, _: Option<&str>) -> Result<(), DomainError> { Ok(()) }
    async fn set_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn set_rename_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn set_messages_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn set_opt_out(&self, _: &str, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn is_opted_out(&self, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn list_opt_outs(&self, _: &str) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
}

fn build(pool: PgPool) -> ManageWalletService {
    let repo = Arc::new(PgWalletRepository::new(pool));
    ManageWalletService::new(repo, Arc::new(TestTaunts))
}

// ── Mode standalone : credit / debit / transfer / get_balance ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credit_creates_wallet_and_returns_mutation() {
    let pool = pool().await;
    let wallet_repo = PgWalletRepository::new(pool.clone());
    let g = fresh_id();
    let u = fresh_id();
    wallet_repo.get_or_create(&g, &u, "Alice", 500).await.unwrap();

    let svc = build(pool);
    let m = svc.credit(&g, &u, 200, "test", "d").await.unwrap();
    assert_eq!(m.previous_balance, 500);
    assert_eq!(m.new_balance, 700);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credit_triggers_jackpot_at_threshold() {
    let pool = pool().await;
    let wallet_repo = PgWalletRepository::new(pool.clone());
    let g = fresh_id();
    let u = fresh_id();
    wallet_repo.get_or_create(&g, &u, "Alice", 100).await.unwrap();
    let svc = build(pool);
    let m = svc.credit(&g, &u, 15_000, "test", "d").await.unwrap();
    assert_eq!(m.triggered_taunts.len(), 1);
    assert_eq!(m.triggered_taunts[0].streak_kind, "jackpot");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debit_to_zero_triggers_bankruptcy() {
    let pool = pool().await;
    let wallet_repo = PgWalletRepository::new(pool.clone());
    let g = fresh_id();
    let u = fresh_id();
    wallet_repo.get_or_create(&g, &u, "Alice", 500).await.unwrap();
    let svc = build(pool);
    let m = svc.debit(&g, &u, 500, "test", "d").await.unwrap();
    assert_eq!(m.new_balance, 0);
    assert_eq!(m.triggered_taunts.len(), 1);
    assert_eq!(m.triggered_taunts[0].streak_kind, "bankruptcy");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_triggers_bankruptcy_sender_and_jackpot_receiver() {
    let pool = pool().await;
    let wallet_repo = PgWalletRepository::new(pool.clone());
    let g = fresh_id();
    let alice = fresh_id();
    let bob = fresh_id();
    wallet_repo.get_or_create(&g, &alice, "Alice", 15_000).await.unwrap();
    wallet_repo.get_or_create(&g, &bob, "Bob", 0).await.unwrap();
    let svc = build(pool);
    let events = svc.transfer(&g, &alice, &bob, 15_000, "gift", "d").await.unwrap();
    assert_eq!(events.len(), 2);
    assert!(events.iter().any(|e| e.streak_kind == "bankruptcy"));
    assert!(events.iter().any(|e| e.streak_kind == "jackpot"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_balance_returns_current_coins() {
    let pool = pool().await;
    let wallet_repo = PgWalletRepository::new(pool.clone());
    let g = fresh_id();
    let u = fresh_id();
    wallet_repo.get_or_create(&g, &u, "Alice", 1234).await.unwrap();
    let svc = build(pool);
    assert_eq!(svc.get_balance(&g, &u).await.unwrap(), 1234);
}

// ── Mode tx : credit_tx / debit_tx / post_commit_taunts ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credit_tx_updates_balance_and_reports_jackpot_flag() {
    let pool = pool().await;
    let wallet_repo = PgWalletRepository::new(pool.clone());
    let g = fresh_id();
    let u = fresh_id();
    wallet_repo.get_or_create(&g, &u, "Alice", 100).await.unwrap();
    let svc = build(pool.clone());

    let mut tx = pool.begin().await.unwrap();
    let mutation = svc.credit_tx(&mut tx, &g, &u, 20_000, "test", "d").await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(mutation.previous_balance, 100);
    assert_eq!(mutation.new_balance, 20_100);
    assert_eq!(mutation.maybe_jackpot_amount, Some(20_000));
    assert!(!mutation.maybe_bankruptcy);

    let events = svc.post_commit_taunts(&g, &u, &mutation).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].streak_kind, "jackpot");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debit_tx_to_zero_marks_bankruptcy() {
    let pool = pool().await;
    let wallet_repo = PgWalletRepository::new(pool.clone());
    let g = fresh_id();
    let u = fresh_id();
    wallet_repo.get_or_create(&g, &u, "Alice", 500).await.unwrap();
    let svc = build(pool.clone());

    let mut tx = pool.begin().await.unwrap();
    let mutation = svc.debit_tx(&mut tx, &g, &u, 500, "test", "d").await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(mutation.new_balance, 0);
    assert!(mutation.maybe_bankruptcy);
    let events = svc.post_commit_taunts(&g, &u, &mutation).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].streak_kind, "bankruptcy");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debit_tx_rejects_insufficient_balance() {
    let pool = pool().await;
    let wallet_repo = PgWalletRepository::new(pool.clone());
    let g = fresh_id();
    let u = fresh_id();
    wallet_repo.get_or_create(&g, &u, "Alice", 50).await.unwrap();
    let svc = build(pool.clone());

    let mut tx = pool.begin().await.unwrap();
    let err = svc.debit_tx(&mut tx, &g, &u, 500, "test", "d").await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credit_tx_unknown_wallet_returns_not_found() {
    let pool = pool().await;
    let svc = build(pool.clone());
    let mut tx = pool.begin().await.unwrap();
    let err = svc.credit_tx(&mut tx, &fresh_id(), &fresh_id(), 100, "t", "d").await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debit_tx_unknown_wallet_returns_not_found() {
    let pool = pool().await;
    let svc = build(pool.clone());
    let mut tx = pool.begin().await.unwrap();
    let err = svc.debit_tx(&mut tx, &fresh_id(), &fresh_id(), 100, "t", "d").await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credit_tx_rejects_non_positive_amount() {
    let pool = pool().await;
    let svc = build(pool.clone());
    let mut tx = pool.begin().await.unwrap();
    assert!(svc.credit_tx(&mut tx, "g", "u", 0, "t", "d").await.is_err());
    assert!(svc.credit_tx(&mut tx, "g", "u", -5, "t", "d").await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debit_tx_rejects_non_positive_amount() {
    let pool = pool().await;
    let svc = build(pool.clone());
    let mut tx = pool.begin().await.unwrap();
    assert!(svc.debit_tx(&mut tx, "g", "u", 0, "t", "d").await.is_err());
    assert!(svc.debit_tx(&mut tx, "g", "u", -5, "t", "d").await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn post_commit_taunts_empty_when_flags_unset() {
    let pool = pool().await;
    let svc = build(pool);
    let mutation = TxWalletMutation {
        new_balance: 100, previous_balance: 0,
        maybe_bankruptcy: false, maybe_jackpot_amount: None,
    };
    let events = svc.post_commit_taunts("g", "u", &mutation).await;
    assert!(events.is_empty());
}
