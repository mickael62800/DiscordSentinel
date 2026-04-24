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

// ── pay_combat_atomic ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pay_combat_atomic_transfers_from_loser_to_winner() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let w = fresh_id(); let l = fresh_id();
    repo.get_or_create(&g, &w, "Winner", 100).await.unwrap();
    repo.get_or_create(&g, &l, "Loser", 500).await.unwrap();
    repo.pay_combat_atomic(&g, &w, 200, &l, 200, "coude_combat", "test").await.unwrap();
    let winner = repo.get(&g, &w).await.unwrap().unwrap();
    let loser = repo.get(&g, &l).await.unwrap().unwrap();
    assert_eq!(winner.coins, 300);
    assert_eq!(loser.coins, 300);
    // Deux transactions persistees (une par wallet).
    let w_tx = repo.get_transactions(&g, &w, 10).await.unwrap();
    let l_tx = repo.get_transactions(&g, &l, 10).await.unwrap();
    assert_eq!(w_tx.len(), 1);
    assert_eq!(w_tx[0].amount, 200);
    assert_eq!(l_tx.len(), 1);
    assert_eq!(l_tx[0].amount, -200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pay_combat_atomic_clamps_loser_to_zero() {
    // Loser a 50 coins, on debite 200 → clamp a 0 via GREATEST(coins - 200, 0).
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let w = fresh_id(); let l = fresh_id();
    repo.get_or_create(&g, &w, "W", 0).await.unwrap();
    repo.get_or_create(&g, &l, "L", 50).await.unwrap();
    repo.pay_combat_atomic(&g, &w, 200, &l, 200, "test", "x").await.unwrap();
    let loser = repo.get(&g, &l).await.unwrap().unwrap();
    assert_eq!(loser.coins, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pay_combat_atomic_only_winner_no_loser() {
    // loser_amount = 0 → branche skip debit; winner credit seul.
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let w = fresh_id();
    repo.get_or_create(&g, &w, "W", 100).await.unwrap();
    repo.pay_combat_atomic(&g, &w, 50, "ghost", 0, "test", "x").await.unwrap();
    let winner = repo.get(&g, &w).await.unwrap().unwrap();
    assert_eq!(winner.coins, 150);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pay_combat_atomic_only_loser_no_winner() {
    // winner_amount = 0 → branche skip credit; loser debit seul.
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let l = fresh_id();
    repo.get_or_create(&g, &l, "L", 200).await.unwrap();
    repo.pay_combat_atomic(&g, "ghost", 0, &l, 100, "test", "x").await.unwrap();
    let loser = repo.get(&g, &l).await.unwrap().unwrap();
    assert_eq!(loser.coins, 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pay_combat_atomic_missing_wallets_silent_skip() {
    // Wallet inexistant : le repo ne fail pas (le combat est deja resolu en domain).
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id();
    let res = repo.pay_combat_atomic(&g, "nowinner", 100, "noloser", 100, "test", "x").await;
    assert!(res.is_ok());
}

// ── credit / debit / transfer : paths not_found ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credit_unknown_wallet_returns_not_found() {
    let repo = PgWalletRepository::new(pool().await);
    let err = repo.credit(&fresh_id(), &fresh_id(), 100, "s", "d").await.unwrap_err();
    assert!(matches!(err, sentinel_api::domain::errors::DomainError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debit_unknown_wallet_returns_not_found() {
    let repo = PgWalletRepository::new(pool().await);
    let err = repo.debit(&fresh_id(), &fresh_id(), 100, "s", "d").await.unwrap_err();
    assert!(matches!(err, sentinel_api::domain::errors::DomainError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_unknown_sender_returns_not_found() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id();
    let b = fresh_id();
    repo.get_or_create(&g, &b, "B", 100).await.unwrap();
    let err = repo.transfer(&g, "ghost", &b, 50, "s", "d").await.unwrap_err();
    assert!(matches!(err, sentinel_api::domain::errors::DomainError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_unknown_receiver_returns_not_found() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id();
    let a = fresh_id();
    repo.get_or_create(&g, &a, "A", 500).await.unwrap();
    let err = repo.transfer(&g, &a, "ghost", 100, "s", "d").await.unwrap_err();
    assert!(matches!(err, sentinel_api::domain::errors::DomainError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_insufficient_balance_returns_validation_error() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let a = fresh_id(); let b = fresh_id();
    repo.get_or_create(&g, &a, "A", 50).await.unwrap();
    repo.get_or_create(&g, &b, "B", 0).await.unwrap();
    let err = repo.transfer(&g, &a, &b, 500, "s", "d").await.unwrap_err();
    assert!(matches!(err, sentinel_api::domain::errors::DomainError::ValidationError(_)));
    // Aucun debit effectue (rollback).
    let wa = repo.get(&g, &a).await.unwrap().unwrap();
    assert_eq!(wa.coins, 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_wallet_unknown_returns_not_found() {
    let repo = PgWalletRepository::new(pool().await);
    let err = repo.reset_wallet(&fresh_id(), &fresh_id(), 100).await.unwrap_err();
    assert!(matches!(err, sentinel_api::domain::errors::DomainError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_all_wallets_on_empty_guild_returns_zero() {
    let repo = PgWalletRepository::new(pool().await);
    let n = repo.reset_all_wallets(&fresh_id(), 100).await.unwrap();
    assert_eq!(n, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credit_writes_wallet_transaction_with_positive_amount() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A", 0).await.unwrap();
    repo.credit(&g, &u, 250, "bonus", "test-credit").await.unwrap();
    let txs = repo.get_transactions(&g, &u, 10).await.unwrap();
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].amount, 250);
    assert_eq!(txs[0].balance_after, 250);
    assert_eq!(txs[0].source, "bonus");
    assert_eq!(txs[0].description, "test-credit");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debit_writes_wallet_transaction_with_negative_amount() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A", 1000).await.unwrap();
    repo.debit(&g, &u, 300, "shop", "item").await.unwrap();
    let txs = repo.get_transactions(&g, &u, 10).await.unwrap();
    assert_eq!(txs[0].amount, -300);
    assert_eq!(txs[0].balance_after, 700);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_transactions_limit_clamps_results() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.get_or_create(&g, &u, "A", 0).await.unwrap();
    for i in 0..5 {
        repo.credit(&g, &u, 10, "s", &format!("t{i}")).await.unwrap();
    }
    let txs = repo.get_transactions(&g, &u, 3).await.unwrap();
    assert_eq!(txs.len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_by_guild_sorted_by_coins_desc() {
    let repo = PgWalletRepository::new(pool().await);
    let g = fresh_id();
    let u1 = fresh_id(); let u2 = fresh_id(); let u3 = fresh_id();
    repo.get_or_create(&g, &u1, "A", 100).await.unwrap();
    repo.get_or_create(&g, &u2, "B", 500).await.unwrap();
    repo.get_or_create(&g, &u3, "C", 200).await.unwrap();
    let all = repo.list_by_guild(&g).await.unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].coins, 500);
    assert_eq!(all[1].coins, 200);
    assert_eq!(all[2].coins, 100);
}
