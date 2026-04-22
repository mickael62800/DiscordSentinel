//! Tests d'integration postgres pour PgCoudeCashboxRepository.

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::PgCoudeCashboxRepository;
use sentinel_api::domain::entities::CashboxSource;
use sentinel_api::ports::outbound::CoudeCashboxRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_or_create_initializes_balance_zero() {
    let repo = PgCoudeCashboxRepository::new(pool().await);
    let g = fresh_id();
    let cb = repo.get_or_create(&g).await.unwrap();
    assert_eq!(cb.guild_id, g);
    assert_eq!(cb.balance, 0);
    assert_eq!(cb.total_collected, 0);
    assert_eq!(cb.total_redistributed, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deposit_increases_balance_and_total_collected() {
    let repo = PgCoudeCashboxRepository::new(pool().await);
    let g = fresh_id();
    repo.deposit(&g, 100, CashboxSource::ShopPurchase).await.unwrap();
    repo.deposit(&g, 200, CashboxSource::BetCommission).await.unwrap();
    let cb = repo.get_or_create(&g).await.unwrap();
    assert_eq!(cb.balance, 300);
    assert_eq!(cb.total_collected, 300);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deposit_creates_cashbox_if_absent() {
    let repo = PgCoudeCashboxRepository::new(pool().await);
    let g = fresh_id();
    // deposit sans get_or_create prealable.
    repo.deposit(&g, 50, CashboxSource::DonationTax).await.unwrap();
    let cb = repo.get_or_create(&g).await.unwrap();
    assert_eq!(cb.balance, 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn withdraw_clamps_to_balance() {
    let repo = PgCoudeCashboxRepository::new(pool().await);
    let g = fresh_id();
    repo.deposit(&g, 100, CashboxSource::ShopPurchase).await.unwrap();
    // Demande 300 -> clamp a 100 disponible
    let actual = repo.withdraw(&g, 300).await.unwrap();
    assert_eq!(actual, 100);
    let cb = repo.get_or_create(&g).await.unwrap();
    assert_eq!(cb.balance, 0);
    // total_collected ne bouge pas (withdraw != deposit)
    assert_eq!(cb.total_collected, 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn withdraw_partial_amount() {
    let repo = PgCoudeCashboxRepository::new(pool().await);
    let g = fresh_id();
    repo.deposit(&g, 500, CashboxSource::ShopPurchase).await.unwrap();
    let actual = repo.withdraw(&g, 200).await.unwrap();
    assert_eq!(actual, 200);
    assert_eq!(repo.get_or_create(&g).await.unwrap().balance, 300);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claim_all_for_redistribution_empties_cashbox() {
    let repo = PgCoudeCashboxRepository::new(pool().await);
    let g = fresh_id();
    repo.deposit(&g, 750, CashboxSource::BetCommission).await.unwrap();
    let claimed = repo.claim_all_for_redistribution(&g).await.unwrap();
    assert_eq!(claimed, 750);
    assert_eq!(repo.get_or_create(&g).await.unwrap().balance, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claim_all_empty_cashbox_returns_zero() {
    let repo = PgCoudeCashboxRepository::new(pool().await);
    let g = fresh_id();
    assert_eq!(repo.claim_all_for_redistribution(&g).await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_redistribution_persists_entries() {
    let repo = PgCoudeCashboxRepository::new(pool().await);
    let g = fresh_id();
    let entries = vec![
        ("u1".to_string(), "Alice".to_string(), 100),
        ("u2".to_string(), "Bob".to_string(), 50),
    ];
    let redistrib_id = repo.record_redistribution(&g, 150, entries).await.unwrap();
    // List redistributions
    let list = repo.list_redistributions(&g, 10).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].total_amount, 150);
    assert_eq!(list[0].winners_count, 2);
    // List entries
    let entries = repo.list_entries(redistrib_id).await.unwrap();
    assert_eq!(entries.len(), 2);
    let total: i64 = entries.iter().map(|e| e.amount_won).sum();
    assert_eq!(total, 150);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_redistributions_empty_when_none() {
    let repo = PgCoudeCashboxRepository::new(pool().await);
    assert!(repo.list_redistributions(&fresh_id(), 10).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_active_players_empty_when_no_combat() {
    let repo = PgCoudeCashboxRepository::new(pool().await);
    assert!(repo.list_active_players(&fresh_id(), 7).await.unwrap().is_empty());
}
