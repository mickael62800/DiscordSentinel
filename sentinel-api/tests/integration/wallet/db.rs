//! Tests d'integration REELS pour le wallet (avec PostgreSQL).
//! Necessite : DATABASE_URL pointant vers une base de test avec migrations appliquees.
//! Lancer : cargo test --test integration_wallet

use sentinel_api::adapters::outbound::postgres::casino::wallet_repository::PgWalletRepository;
use sentinel_api::ports::outbound::casino::wallet_repository::WalletRepository;
use sqlx::PgPool;

async fn setup_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url)
        .await
        .expect("Impossible de se connecter a la base de test")
}

/// Genere un guild_id unique pour isoler chaque test.
fn unique_guild() -> String {
    format!(
        "{}",
        uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    )
}

// ══════════════════════════════════════════════════════════
//  Wallet CRUD
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn wallet_create_and_get() {
    let pool = setup_pool().await;
    let repo = PgWalletRepository::new(pool);
    let gid = unique_guild();

    let wallet = repo
        .get_or_create(&gid, "user1", "Alice", 100)
        .await
        .unwrap();
    assert_eq!(wallet.coins, 100);
    assert_eq!(wallet.username, "Alice");

    let fetched = repo.get(&gid, "user1").await.unwrap().unwrap();
    assert_eq!(fetched.coins, 100);
}

#[tokio::test]
async fn wallet_credit_increases_balance() {
    let pool = setup_pool().await;
    let repo = PgWalletRepository::new(pool);
    let gid = unique_guild();

    repo.get_or_create(&gid, "user1", "Alice", 100)
        .await
        .unwrap();
    let wallet = repo
        .credit(&gid, "user1", 50, "test", "bonus")
        .await
        .unwrap();
    assert_eq!(wallet.coins, 150);
    assert_eq!(wallet.total_earned, 50);
}

#[tokio::test]
async fn wallet_debit_decreases_balance() {
    let pool = setup_pool().await;
    let repo = PgWalletRepository::new(pool);
    let gid = unique_guild();

    repo.get_or_create(&gid, "user1", "Alice", 100)
        .await
        .unwrap();
    let wallet = repo
        .debit(&gid, "user1", 30, "test", "achat")
        .await
        .unwrap();
    assert_eq!(wallet.coins, 70);
    assert_eq!(wallet.total_spent, 30);
}

#[tokio::test]
async fn wallet_debit_insufficient_balance_rejected() {
    let pool = setup_pool().await;
    let repo = PgWalletRepository::new(pool);
    let gid = unique_guild();

    repo.get_or_create(&gid, "user1", "Alice", 50)
        .await
        .unwrap();
    let result = repo.debit(&gid, "user1", 100, "test", "trop cher").await;
    assert!(result.is_err());

    // Verifier que le solde n'a pas change
    let wallet = repo.get(&gid, "user1").await.unwrap().unwrap();
    assert_eq!(wallet.coins, 50);
}

#[tokio::test]
async fn wallet_debit_exact_balance_succeeds() {
    let pool = setup_pool().await;
    let repo = PgWalletRepository::new(pool);
    let gid = unique_guild();

    repo.get_or_create(&gid, "user1", "Alice", 100)
        .await
        .unwrap();
    let wallet = repo
        .debit(&gid, "user1", 100, "test", "tout")
        .await
        .unwrap();
    assert_eq!(wallet.coins, 0);
}

// ══════════════════════════════════════════════════════════
//  Transfer — verifier qu'on ne cree pas de coins
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn wallet_transfer_moves_coins() {
    let pool = setup_pool().await;
    let repo = PgWalletRepository::new(pool);
    let gid = unique_guild();

    repo.get_or_create(&gid, "sender", "Bob", 200)
        .await
        .unwrap();
    repo.get_or_create(&gid, "receiver", "Alice", 50)
        .await
        .unwrap();

    repo.transfer(&gid, "sender", "receiver", 75, "test", "cadeau")
        .await
        .unwrap();

    let sender = repo.get(&gid, "sender").await.unwrap().unwrap();
    let receiver = repo.get(&gid, "receiver").await.unwrap().unwrap();

    assert_eq!(sender.coins, 125);
    assert_eq!(receiver.coins, 125);
    // Total coins dans le systeme = 250 (identique a avant)
    assert_eq!(sender.coins + receiver.coins, 250);
}

#[tokio::test]
async fn wallet_transfer_insufficient_balance_rejected() {
    let pool = setup_pool().await;
    let repo = PgWalletRepository::new(pool);
    let gid = unique_guild();

    repo.get_or_create(&gid, "sender", "Bob", 30).await.unwrap();
    repo.get_or_create(&gid, "receiver", "Alice", 50)
        .await
        .unwrap();

    let result = repo
        .transfer(&gid, "sender", "receiver", 100, "test", "trop")
        .await;
    assert!(result.is_err());

    // Rien n'a bouge
    let sender = repo.get(&gid, "sender").await.unwrap().unwrap();
    let receiver = repo.get(&gid, "receiver").await.unwrap().unwrap();
    assert_eq!(sender.coins, 30);
    assert_eq!(receiver.coins, 50);
}

// ══════════════════════════════════════════════════════════
//  Transactions — audit trail
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn wallet_transactions_logged() {
    let pool = setup_pool().await;
    let repo = PgWalletRepository::new(pool);
    let gid = unique_guild();

    repo.get_or_create(&gid, "user1", "Alice", 100)
        .await
        .unwrap();
    repo.credit(&gid, "user1", 50, "blackjack", "gain")
        .await
        .unwrap();
    repo.debit(&gid, "user1", 20, "coude", "mise")
        .await
        .unwrap();

    let txs = repo.get_transactions(&gid, "user1", 10).await.unwrap();
    assert_eq!(txs.len(), 2);

    // Derniere transaction en premier (DESC)
    assert_eq!(txs[0].amount, -20);
    assert_eq!(txs[0].source, "coude");
    assert_eq!(txs[1].amount, 50);
    assert_eq!(txs[1].source, "blackjack");
}

// ══════════════════════════════════════════════════════════
//  Leaderboard
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn wallet_leaderboard_sorted_by_coins() {
    let pool = setup_pool().await;
    let repo = PgWalletRepository::new(pool.clone());
    let gid = unique_guild();

    repo.get_or_create(&gid, "poor", "Poor", 10).await.unwrap();
    repo.get_or_create(&gid, "rich", "Rich", 1000)
        .await
        .unwrap();
    repo.get_or_create(&gid, "mid", "Mid", 500).await.unwrap();

    // leaderboard() lit depuis la materialized view mv_wallet_leaderboard
    // qui est refreshee toutes les 5 min par le cache-worker en prod.
    // En test, on force le refresh pour voir les donnees fraiches.
    sqlx::query("REFRESH MATERIALIZED VIEW mv_wallet_leaderboard")
        .execute(&pool)
        .await
        .unwrap();

    let lb = repo.leaderboard(&gid, 10).await.unwrap();
    assert_eq!(lb.len(), 3);
    assert_eq!(lb[0].username, "Rich");
    assert_eq!(lb[1].username, "Mid");
    assert_eq!(lb[2].username, "Poor");
}
