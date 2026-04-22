//! Tests d'integration postgres pour PgBlackjackRepository.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::PgBlackjackRepository;
use sentinel_api::domain::entities::{BlackjackGame, Card};
use sentinel_api::ports::outbound::BlackjackRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}
fn card(rank: &str, suit: &str) -> Card {
    Card { rank: rank.into(), suit: suit.into() }
}
fn sample(guild: &str, user: &str, status: &str) -> BlackjackGame {
    BlackjackGame {
        id: Uuid::new_v4(),
        guild_id: guild.into(), user_id: user.into(), username: user.into(),
        bet: 100,
        player_hand: vec![card("A", "spades"), card("10", "hearts")],
        dealer_hand: vec![card("5", "clubs")],
        deck: (0..20).map(|i| card(&format!("{}", i % 10 + 2), "diamonds")).collect(),
        status: status.into(),
        player_score: 21, dealer_score: 5,
        doubled: false, payout: 0,
        created_at: Utc::now(), finished_at: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_and_get_by_id() {
    let repo = PgBlackjackRepository::new(pool().await);
    let g = fresh_id();
    let game = sample(&g, &fresh_id(), "playing");
    repo.create(&game).await.unwrap();
    let got = repo.get_by_id(game.id).await.unwrap().unwrap();
    assert_eq!(got.bet, 100);
    assert_eq!(got.player_hand.len(), 2);
    assert_eq!(got.player_score, 21);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_active_returns_only_active_game() {
    let repo = PgBlackjackRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    // Cree une partie active
    let active = sample(&g, &u, "playing");
    repo.create(&active).await.unwrap();
    let got = repo.get_active(&g, &u).await.unwrap().unwrap();
    assert_eq!(got.id, active.id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_active_none_when_absent() {
    let repo = PgBlackjackRepository::new(pool().await);
    assert!(repo.get_active(&fresh_id(), &fresh_id()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_persists_status_and_payout() {
    let repo = PgBlackjackRepository::new(pool().await);
    let g = fresh_id();
    let mut game = sample(&g, &fresh_id(), "playing");
    repo.create(&game).await.unwrap();
    game.status = "player_win".into();
    game.payout = 250;
    game.finished_at = Some(Utc::now());
    repo.update(&game).await.unwrap();
    let got = repo.get_by_id(game.id).await.unwrap().unwrap();
    assert_eq!(got.status, "player_win");
    assert_eq!(got.payout, 250);
    assert!(got.finished_at.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_by_guild_no_filter() {
    let repo = PgBlackjackRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.create(&sample(&g, &u, "playing")).await.unwrap();
    // 2eme partie avec user different (sinon violation unique active).
    let mut done = sample(&g, &fresh_id(), "player_win");
    done.finished_at = Some(Utc::now());
    repo.create(&done).await.unwrap();
    let all = repo.list_by_guild(&g, None).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_by_guild_filter_by_status() {
    let repo = PgBlackjackRepository::new(pool().await);
    let g = fresh_id();
    let mut won = sample(&g, &fresh_id(), "player_win");
    won.finished_at = Some(Utc::now());
    repo.create(&won).await.unwrap();
    let mut lost = sample(&g, &fresh_id(), "dealer_win");
    lost.finished_at = Some(Utc::now());
    repo.create(&lost).await.unwrap();

    let wins = repo.list_by_guild(&g, Some("player_win")).await.unwrap();
    assert_eq!(wins.len(), 1);
    assert_eq!(wins[0].id, won.id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_game_marks_cancelled() {
    // cancel_game refunde la mise via wallet_transactions, donc il faut un
    // wallet existant pour le user.
    let p = pool().await;
    let repo = PgBlackjackRepository::new(p.clone());
    let g = fresh_id();
    let u = fresh_id();
    sqlx::query("INSERT INTO user_wallets (guild_id, user_id, username, coins) VALUES ($1, $2, 'Alice', 1000)")
        .bind(&g).bind(&u).execute(&p).await.unwrap();

    let game = sample(&g, &u, "playing");
    repo.create(&game).await.unwrap();
    repo.cancel_game(game.id).await.unwrap();
    let got = repo.get_by_id(game.id).await.unwrap().unwrap();
    assert_eq!(got.status, "cancelled");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_by_id_unknown_returns_none() {
    let repo = PgBlackjackRepository::new(pool().await);
    assert!(repo.get_by_id(Uuid::new_v4()).await.unwrap().is_none());
}
