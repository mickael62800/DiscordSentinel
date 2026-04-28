//! Tests d'integration postgres pour PgBlackjackRepository.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::casino::blackjack_repository::PgBlackjackRepository;
use sentinel_api::domain::entities::casino::blackjack::BlackjackGame;
use sentinel_api::domain::entities::casino::blackjack::Card;
use sentinel_api::ports::outbound::casino::blackjack_repository::BlackjackRepository;

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_game_refunds_bet_to_wallet() {
    let p = pool().await;
    let repo = PgBlackjackRepository::new(p.clone());
    let g = fresh_id();
    let u = fresh_id();
    sqlx::query("INSERT INTO user_wallets (guild_id, user_id, username, coins) VALUES ($1, $2, 'Alice', 500)")
        .bind(&g).bind(&u).execute(&p).await.unwrap();

    let mut game = sample(&g, &u, "playing");
    game.bet = 75;
    repo.create(&game).await.unwrap();
    repo.cancel_game(game.id).await.unwrap();

    let coins: i64 = sqlx::query_scalar("SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2")
        .bind(&g).bind(&u).fetch_one(&p).await.unwrap();
    assert_eq!(coins, 575); // 500 + 75 refund
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_game_doubled_refunds_double_bet() {
    let p = pool().await;
    let repo = PgBlackjackRepository::new(p.clone());
    let g = fresh_id();
    let u = fresh_id();
    sqlx::query("INSERT INTO user_wallets (guild_id, user_id, username, coins) VALUES ($1, $2, 'A', 100)")
        .bind(&g).bind(&u).execute(&p).await.unwrap();
    let mut game = sample(&g, &u, "playing");
    game.bet = 50;
    game.doubled = true;
    repo.create(&game).await.unwrap();
    repo.cancel_game(game.id).await.unwrap();
    let coins: i64 = sqlx::query_scalar("SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2")
        .bind(&g).bind(&u).fetch_one(&p).await.unwrap();
    // 100 + (50 + 50 double) = 200
    assert_eq!(coins, 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_game_writes_audit_transaction() {
    let p = pool().await;
    let repo = PgBlackjackRepository::new(p.clone());
    let g = fresh_id();
    let u = fresh_id();
    sqlx::query("INSERT INTO user_wallets (guild_id, user_id, username, coins) VALUES ($1, $2, 'A', 100)")
        .bind(&g).bind(&u).execute(&p).await.unwrap();
    let game = sample(&g, &u, "playing");
    repo.create(&game).await.unwrap();
    repo.cancel_game(game.id).await.unwrap();
    let src: String = sqlx::query_scalar(
        "SELECT source FROM wallet_transactions WHERE guild_id = $1 AND user_id = $2 ORDER BY created_at DESC LIMIT 1"
    ).bind(&g).bind(&u).fetch_one(&p).await.unwrap();
    assert_eq!(src, "blackjack_cancel");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_game_not_found_returns_error() {
    let repo = PgBlackjackRepository::new(pool().await);
    let err = repo.cancel_game(Uuid::new_v4()).await.unwrap_err();
    assert!(matches!(err, sentinel_api::domain::errors::DomainError::NotFound(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_game_already_terminated_returns_conflict() {
    let p = pool().await;
    let repo = PgBlackjackRepository::new(p.clone());
    let g = fresh_id();
    let u = fresh_id();
    sqlx::query("INSERT INTO user_wallets (guild_id, user_id, username, coins) VALUES ($1, $2, 'A', 100)")
        .bind(&g).bind(&u).execute(&p).await.unwrap();
    let game = sample(&g, &u, "dealer_wins"); // status terminé
    repo.create(&game).await.unwrap();
    let err = repo.cancel_game(game.id).await.unwrap_err();
    assert!(matches!(err, sentinel_api::domain::errors::DomainError::Conflict(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_game_waiting_status_also_cancellable() {
    // Status "waiting" est aussi cancellable (multi-table mode).
    let p = pool().await;
    let repo = PgBlackjackRepository::new(p.clone());
    let g = fresh_id();
    let u = fresh_id();
    sqlx::query("INSERT INTO user_wallets (guild_id, user_id, username, coins) VALUES ($1, $2, 'A', 100)")
        .bind(&g).bind(&u).execute(&p).await.unwrap();
    let game = sample(&g, &u, "waiting");
    repo.create(&game).await.unwrap();
    assert!(repo.cancel_game(game.id).await.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_already_terminated_returns_conflict() {
    let p = pool().await;
    let repo = PgBlackjackRepository::new(p.clone());
    let g = fresh_id(); let u = fresh_id();
    let mut game = sample(&g, &u, "player_wins"); // pas playing
    game.finished_at = Some(Utc::now());
    repo.create(&game).await.unwrap();
    // Update avec id existant mais status != playing → conflict
    game.status = "player_bust".into();
    let err = repo.update(&game).await.unwrap_err();
    assert!(matches!(err, sentinel_api::domain::errors::DomainError::Conflict(_)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_by_guild_empty_returns_empty() {
    let repo = PgBlackjackRepository::new(pool().await);
    let g = fresh_id();
    let list = repo.list_by_guild(&g, None).await.unwrap();
    assert!(list.is_empty());
    let list2 = repo.list_by_guild(&g, Some("playing")).await.unwrap();
    assert!(list2.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_active_excludes_stale_games() {
    // Partie creee > 30 min → get_active doit retourner None.
    let p = pool().await;
    let repo = PgBlackjackRepository::new(p.clone());
    let g = fresh_id(); let u = fresh_id();
    let mut game = sample(&g, &u, "playing");
    // Backdate created_at au-dela de 30 min
    game.created_at = Utc::now() - chrono::Duration::hours(1);
    repo.create(&game).await.unwrap();
    let active = repo.get_active(&g, &u).await.unwrap();
    assert!(active.is_none(), "partie > 30 min ne devrait plus etre active");
}
