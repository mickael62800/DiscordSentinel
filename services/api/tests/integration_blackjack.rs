//! Tests d'integration REELS pour le blackjack (avec PostgreSQL).
//! Verifie les contraintes d'integrite : unique active game, status guard sur update.

use std::sync::Arc;

use sqlx::PgPool;
use sentinel_api::adapters::outbound::postgres::{PgBlackjackRepository, PgWalletRepository};
use sentinel_api::application::BlackjackService;
use sentinel_api::ports::outbound::WalletRepository;

async fn setup_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.expect("Impossible de se connecter a la base de test")
}

fn unique_guild() -> String {
    format!("{}", uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

fn build_service(pool: PgPool) -> BlackjackService {
    let bj_repo = Arc::new(PgBlackjackRepository::new(pool.clone()));
    let wallet_repo = Arc::new(PgWalletRepository::new(pool));
    BlackjackService::new(bj_repo, wallet_repo)
}

// ══════════════════════════════════════════════════════════
//  Game lifecycle
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn blackjack_start_game_debits_wallet() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo.get_or_create(&gid, "player1", "Alice", 500).await.unwrap();

    let svc = build_service(pool.clone());
    let game = svc.start_game(gid.clone(), "player1".into(), "Alice".into(), 100, 10, 10000, 500, 1.5).await.unwrap();
    assert!(game.player_score > 0);

    // Wallet debite de 100 (sauf si blackjack naturel ou le payout est deja credite)
    let wallet = wallet_repo.get(&gid, "player1").await.unwrap().unwrap();
    if game.status == "player_blackjack" {
        // Blackjack naturel : debite 100 puis credite 250 = net +150
        assert_eq!(wallet.coins, 650);
    } else {
        assert_eq!(wallet.coins, 400);
    }
}

#[tokio::test]
async fn blackjack_cannot_start_two_games() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo.get_or_create(&gid, "player1", "Alice", 1000).await.unwrap();

    let svc = build_service(pool.clone());
    let game1 = svc.start_game(gid.clone(), "player1".into(), "Alice".into(), 50, 10, 10000, 1000, 1.5).await;
    assert!(game1.is_ok());

    // Si le premier jeu est encore en cours, le second doit echouer
    let game1 = game1.unwrap();
    if game1.status == "playing" {
        let game2 = svc.start_game(gid.clone(), "player1".into(), "Alice".into(), 50, 10, 10000, 1000, 1.5).await;
        assert!(game2.is_err(), "Devrait refuser une deuxieme partie active");
    }
}

#[tokio::test]
async fn blackjack_hit_and_stand() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo.get_or_create(&gid, "player1", "Alice", 500).await.unwrap();

    let svc = build_service(pool.clone());
    let game = svc.start_game(gid.clone(), "player1".into(), "Alice".into(), 50, 10, 10000, 500, 1.5).await.unwrap();

    if game.status != "playing" {
        return; // Blackjack naturel, pas de hit possible
    }

    // Hit
    let game = svc.hit(game.id).await.unwrap();
    assert!(game.player_hand.len() >= 3);

    if game.status == "playing" {
        // Stand
        let game = svc.stand(game.id).await.unwrap();
        assert_ne!(game.status, "playing", "Le jeu devrait etre termine apres stand");
        assert!(game.dealer_score > 0);
    }
}

#[tokio::test]
async fn blackjack_wallet_balance_conserved() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo.get_or_create(&gid, "player1", "Alice", 500).await.unwrap();

    let svc = build_service(pool.clone());
    let game = svc.start_game(gid.clone(), "player1".into(), "Alice".into(), 100, 10, 10000, 500, 1.5).await.unwrap();

    // Terminer la partie
    let final_game = if game.status == "playing" {
        svc.stand(game.id).await.unwrap()
    } else {
        game
    };

    let wallet = wallet_repo.get(&gid, "player1").await.unwrap().unwrap();

    // Verifier la coherence : solde = 500 - mise + payout
    let expected = 500 - 100 + final_game.payout;
    assert_eq!(wallet.coins, expected, "Solde incoherent : 500 - 100 + {} = {} mais wallet = {}",
        final_game.payout, expected, wallet.coins);
}
