//! Tests d'integration postgres pour PgCoudeEconomyRepository.

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::PgCoudeEconomyRepository;
use sentinel_api::ports::outbound::coude::economy_repository::CoudeEconomyRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

async fn seed_player(p: &PgPool, guild: &str, user: &str) {
    sqlx::query("INSERT INTO coude_players (guild_id, user_id, username) VALUES ($1, $2, 'T')")
        .bind(guild).bind(user).execute(p).await.unwrap();
}
async fn seed_wallet(p: &PgPool, guild: &str, user: &str, coins: i64) {
    sqlx::query("INSERT INTO user_wallets (guild_id, user_id, username, coins) VALUES ($1, $2, 'T', $3)")
        .bind(guild).bind(user).bind(coins).execute(p).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_coins_returns_balance() {
    let p = pool().await;
    let repo = PgCoudeEconomyRepository::new(p.clone());
    let g = fresh_id(); let u = fresh_id();
    seed_wallet(&p, &g, &u, 250).await;
    assert_eq!(repo.get_coins(&g, &u).await.unwrap(), 250);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_coins_not_found_when_no_wallet() {
    let repo = PgCoudeEconomyRepository::new(pool().await);
    let err = repo.get_coins(&fresh_id(), &fresh_id()).await;
    assert!(err.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_steal_stats_updates_both_players() {
    let p = pool().await;
    let repo = PgCoudeEconomyRepository::new(p.clone());
    let g = fresh_id();
    let thief = fresh_id(); let victim = fresh_id();
    seed_player(&p, &g, &thief).await;
    seed_player(&p, &g, &victim).await;
    repo.record_steal_stats(&g, &thief, &victim, 150).await.unwrap();

    let (stolen, earned): (i64, i64) = sqlx::query_as(
        "SELECT total_stolen, total_earned FROM coude_players WHERE guild_id = $1 AND user_id = $2"
    ).bind(&g).bind(&thief).fetch_one(&p).await.unwrap();
    assert_eq!(stolen, 150);
    assert_eq!(earned, 150);
    let (lost,): (i64,) = sqlx::query_as(
        "SELECT total_lost FROM coude_players WHERE guild_id = $1 AND user_id = $2"
    ).bind(&g).bind(&victim).fetch_one(&p).await.unwrap();
    assert_eq!(lost, 150);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_steal_fail_stats_increments_total_lost() {
    let p = pool().await;
    let repo = PgCoudeEconomyRepository::new(p.clone());
    let g = fresh_id(); let u = fresh_id();
    seed_player(&p, &g, &u).await;
    repo.record_steal_fail_stats(&g, &u, 50).await.unwrap();
    let (lost,): (i64,) = sqlx::query_as(
        "SELECT total_lost FROM coude_players WHERE guild_id = $1 AND user_id = $2"
    ).bind(&g).bind(&u).fetch_one(&p).await.unwrap();
    assert_eq!(lost, 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_casino_win_stats() {
    let p = pool().await;
    let repo = PgCoudeEconomyRepository::new(p.clone());
    let g = fresh_id(); let u = fresh_id();
    seed_player(&p, &g, &u).await;
    repo.record_casino_win_stats(&g, &u, 200).await.unwrap();
    let (wins, earned): (i32, i64) = sqlx::query_as(
        "SELECT casino_wins, total_earned FROM coude_players WHERE guild_id = $1 AND user_id = $2"
    ).bind(&g).bind(&u).fetch_one(&p).await.unwrap();
    assert_eq!(wins, 1);
    assert_eq!(earned, 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_casino_loss_stats() {
    let p = pool().await;
    let repo = PgCoudeEconomyRepository::new(p.clone());
    let g = fresh_id(); let u = fresh_id();
    seed_player(&p, &g, &u).await;
    repo.record_casino_loss_stats(&g, &u, 75).await.unwrap();
    let (losses, lost): (i32, i64) = sqlx::query_as(
        "SELECT casino_losses, total_lost FROM coude_players WHERE guild_id = $1 AND user_id = $2"
    ).bind(&g).bind(&u).fetch_one(&p).await.unwrap();
    assert_eq!(losses, 1);
    assert_eq!(lost, 75);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_casino_faillite_returns_total_lost() {
    let p = pool().await;
    let repo = PgCoudeEconomyRepository::new(p.clone());
    let g = fresh_id(); let u = fresh_id();
    seed_player(&p, &g, &u).await;
    repo.record_casino_loss_stats(&g, &u, 100).await.unwrap();
    let total = repo.record_casino_faillite_stats(&g, &u, 500).await.unwrap();
    assert_eq!(total, 600); // 100 + 500
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn count_casino_today_zero_for_fresh_user() {
    let repo = PgCoudeEconomyRepository::new(pool().await);
    assert_eq!(repo.count_casino_today(&fresh_id(), &fresh_id()).await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sum_casino_gains_today_aggregates_positives() {
    let p = pool().await;
    let repo = PgCoudeEconomyRepository::new(p.clone());
    let g = fresh_id(); let u = fresh_id();
    seed_player(&p, &g, &u).await;
    repo.record_casino_win_stats(&g, &u, 100).await.unwrap();
    repo.record_casino_win_stats(&g, &u, 50).await.unwrap();
    repo.record_casino_loss_stats(&g, &u, 30).await.unwrap(); // negatif, ignore
    assert_eq!(repo.sum_casino_gains_today(&g, &u).await.unwrap(), 150);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn count_steal_today_zero_for_fresh_user() {
    let repo = PgCoudeEconomyRepository::new(pool().await);
    assert_eq!(repo.count_steal_today(&fresh_id(), &fresh_id()).await.unwrap(), 0);
}
