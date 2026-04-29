//! Tests d'integration postgres pour PgInventoryRepository.

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::coude::inventory_repository::PgInventoryRepository;
use sentinel_api::domain::entities::coude::inventory::NewCoudePrime;
use sentinel_api::ports::outbound::coude::inventory_repository::InventoryRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}
async fn seed_wallet(p: &PgPool, g: &str, u: &str, coins: i64) {
    sqlx::query("INSERT INTO user_wallets (guild_id, user_id, username, coins) VALUES ($1, $2, 'T', $3) \
                 ON CONFLICT (guild_id, user_id) DO UPDATE SET coins = EXCLUDED.coins")
        .bind(g).bind(u).bind(coins).execute(p).await.unwrap();
}
async fn seed_player(p: &PgPool, g: &str, u: &str) {
    sqlx::query("INSERT INTO coude_players (guild_id, user_id, username) VALUES ($1, $2, 'T') ON CONFLICT DO NOTHING")
        .bind(g).bind(u).execute(p).await.unwrap();
}

// ── Items ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_inventory_empty() {
    let repo = PgInventoryRepository::new(pool().await);
    assert!(repo.list_inventory(&fresh_id(), &fresh_id()).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_item_creates_and_increments() {
    let repo = PgInventoryRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.add_item(&g, &u, "potion").await.unwrap();
    repo.add_item(&g, &u, "potion").await.unwrap();
    let inv = repo.list_inventory(&g, &u).await.unwrap();
    assert_eq!(inv.len(), 1);
    assert_eq!(inv[0].item_key, "potion");
    assert_eq!(inv[0].quantity, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn has_item_true_when_quantity_positive() {
    let repo = PgInventoryRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    assert!(!repo.has_item(&g, &u, "sword").await.unwrap());
    repo.add_item(&g, &u, "sword").await.unwrap();
    assert!(repo.has_item(&g, &u, "sword").await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn use_item_decrements_and_false_when_absent() {
    let repo = PgInventoryRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    assert!(!repo.use_item(&g, &u, "potion").await.unwrap());
    repo.add_item(&g, &u, "potion").await.unwrap();
    repo.add_item(&g, &u, "potion").await.unwrap();
    assert!(repo.use_item(&g, &u, "potion").await.unwrap());
    let inv = repo.list_inventory(&g, &u).await.unwrap();
    assert_eq!(inv[0].quantity, 1);
}

// ── Primes ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prime_create_and_list_active() {
    let p = pool().await;
    let repo = PgInventoryRepository::new(p.clone());
    let g = fresh_id();
    let target = fresh_id();
    let placer = fresh_id();
    seed_wallet(&p, &g, &placer, 10000).await;
    let prime = repo.create_prime(NewCoudePrime {
        guild_id: g.clone(),
        target_id: target.clone(), target_name: "Target".into(),
        placed_by_id: placer.clone(), placed_by_name: "Placer".into(),
        amount: 500,
    }).await.unwrap();
    assert_eq!(prime.amount, 500);
    assert!(!prime.claimed);
    let list = repo.list_active_primes(&g, &target).await.unwrap();
    assert_eq!(list.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prime_claim_returns_total() {
    let p = pool().await;
    let repo = PgInventoryRepository::new(p.clone());
    let g = fresh_id();
    let target = fresh_id();
    let placer = fresh_id();
    let claimer = fresh_id();
    seed_wallet(&p, &g, &placer, 10000).await;
    seed_wallet(&p, &g, &claimer, 0).await;
    seed_player(&p, &g, &claimer).await;
    repo.create_prime(NewCoudePrime {
        guild_id: g.clone(), target_id: target.clone(),
        target_name: "T".into(), placed_by_id: placer.clone(),
        placed_by_name: "P".into(), amount: 300,
    }).await.unwrap();
    repo.create_prime(NewCoudePrime {
        guild_id: g.clone(), target_id: target.clone(),
        target_name: "T".into(), placed_by_id: placer,
        placed_by_name: "P".into(), amount: 200,
    }).await.unwrap();
    let total = repo.claim_primes(&g, &target, &claimer, "Claimer").await.unwrap();
    assert_eq!(total, 500);
    // Plus d'actives.
    assert!(repo.list_active_primes(&g, &target).await.unwrap().is_empty());
}

// ── Assurances ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn insurance_buy_and_get_active() {
    let repo = PgInventoryRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    assert!(repo.buy_insurance(&g, &u, false, 7200).await.unwrap());
    let got = repo.get_active_insurance(&g, &u).await.unwrap().unwrap();
    assert!(!got.is_scam);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn insurance_buy_false_when_active_exists() {
    let repo = PgInventoryRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    assert!(repo.buy_insurance(&g, &u, true, 3600).await.unwrap());
    // 2e achat tandis que 1re active → false
    assert!(!repo.buy_insurance(&g, &u, true, 3600).await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn insurance_duration_zero_falls_back_to_1h() {
    let repo = PgInventoryRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    // duration_seconds <= 0 → fallback 1h.
    assert!(repo.buy_insurance(&g, &u, false, 0).await.unwrap());
    let got = repo.get_active_insurance(&g, &u).await.unwrap().unwrap();
    let delta = got.expires_at.signed_duration_since(chrono::Utc::now()).num_seconds();
    assert!(delta > 3500 && delta <= 3700, "delta={delta}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn insurance_expire_manually() {
    let repo = PgInventoryRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.buy_insurance(&g, &u, true, 3600).await.unwrap();
    let ins = repo.get_active_insurance(&g, &u).await.unwrap().unwrap();
    assert!(repo.expire_insurance(ins.id).await.unwrap());
    assert!(repo.get_active_insurance(&g, &u).await.unwrap().is_none());
    // expire_insurance update sans WHERE active = TRUE, donc 2e appel
    // retourne aussi true (rows_affected=1). Comportement accepte.
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn insurance_get_active_none_when_absent() {
    let repo = PgInventoryRepository::new(pool().await);
    assert!(repo.get_active_insurance(&fresh_id(), &fresh_id()).await.unwrap().is_none());
}
