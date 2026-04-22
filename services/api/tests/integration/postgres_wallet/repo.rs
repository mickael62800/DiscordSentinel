//! Tests d'integration postgres pour PgWalletRepository.

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::PgWalletRepository;
use sentinel_api::ports::outbound::WalletRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_or_create_initializes_with_starting_coins() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    let w = repo.get_or_create(&g, &u, "Alice", 250).await.unwrap();
    assert_eq!(w.coins, 250);
    assert_eq!(w.guild_id, g);
    assert_eq!(w.user_id, u);
    assert_eq!(w.username, "Alice");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_or_create_idempotent_returns_existing() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    let first = repo.get_or_create(&g, &u, "Alice", 100).await.unwrap();
    let second = repo.get_or_create(&g, &u, "ChangedName", 999).await.unwrap();
    // idempotent : meme id, meme solde initial.
    assert_eq!(second.id, first.id);
    assert_eq!(second.coins, 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_returns_none_when_absent() {
    let repo = PgWalletRepository::new(pool().await);
    assert!(repo.get(&fresh_id(), &fresh_id()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credit_increases_coins_and_total_earned() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A", 0).await.unwrap();
    let after = repo.credit(&g, &u, 100, "test", "credit1").await.unwrap();
    assert_eq!(after.coins, 100);
    assert_eq!(after.total_earned, 100);
    let after2 = repo.credit(&g, &u, 50, "test", "credit2").await.unwrap();
    assert_eq!(after2.coins, 150);
    assert_eq!(after2.total_earned, 150);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debit_decreases_coins_and_increments_total_spent() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A", 500).await.unwrap();
    let after = repo.debit(&g, &u, 200, "spend", "x").await.unwrap();
    assert_eq!(after.coins, 300);
    assert_eq!(after.total_spent, 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debit_below_zero_returns_error() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A", 100).await.unwrap();
    // Debit > balance -> devrait erreur.
    let err = repo.debit(&g, &u, 200, "x", "x").await;
    assert!(err.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_moves_coins_atomic() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let a = fresh_id(); let b = fresh_id();
    repo.get_or_create(&g, &a, "A", 500).await.unwrap();
    repo.get_or_create(&g, &b, "B", 100).await.unwrap();
    repo.transfer(&g, &a, &b, 200, "test", "x").await.unwrap();
    let wa = repo.get(&g, &a).await.unwrap().unwrap();
    let wb = repo.get(&g, &b).await.unwrap().unwrap();
    assert_eq!(wa.coins, 300);
    assert_eq!(wb.coins, 300);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_transactions_returns_chronological_desc() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A", 0).await.unwrap();
    repo.credit(&g, &u, 100, "src1", "first").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    repo.credit(&g, &u, 50, "src2", "second").await.unwrap();

    let txs = repo.get_transactions(&g, &u, 10).await.unwrap();
    assert_eq!(txs.len(), 2);
    assert_eq!(txs[0].description, "second"); // DESC
    assert_eq!(txs[0].amount, 50);
    assert_eq!(txs[1].description, "first");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_by_guild_returns_all_wallets() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id();
    for _ in 0..3 {
        repo.get_or_create(&g, &fresh_id(), "X", 50).await.unwrap();
    }
    let all = repo.list_by_guild(&g).await.unwrap();
    assert_eq!(all.len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_wallet_sets_balance_and_clears_history() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A", 0).await.unwrap();
    repo.credit(&g, &u, 500, "a", "b").await.unwrap();
    repo.reset_wallet(&g, &u, 100).await.unwrap();
    let got = repo.get(&g, &u).await.unwrap().unwrap();
    assert_eq!(got.coins, 100);
    // L'historique associe est efface (ou au moins ne contient plus les anciennes credits).
    let txs = repo.get_transactions(&g, &u, 10).await.unwrap();
    assert!(!txs.iter().any(|t| t.description == "b"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_all_wallets_returns_count() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id();
    for _ in 0..3 {
        repo.get_or_create(&g, &fresh_id(), "X", 999).await.unwrap();
    }
    let n = repo.reset_all_wallets(&g, 50).await.unwrap();
    assert_eq!(n, 3);
    let all = repo.list_by_guild(&g).await.unwrap();
    assert!(all.iter().all(|w| w.coins == 50));
}
