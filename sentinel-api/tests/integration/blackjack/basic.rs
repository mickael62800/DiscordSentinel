//! Tests d'integration REELS pour le blackjack (avec PostgreSQL).
//! Verifie les contraintes d'integrite : unique active game, status guard sur update.

use std::sync::Arc;

use async_trait::async_trait;
use sentinel_api::adapters::outbound::postgres::casino::blackjack_repository::PgBlackjackRepository;
use sentinel_api::adapters::outbound::postgres::casino::wallet_repository::PgWalletRepository;
use sentinel_api::adapters::outbound::postgres::community::member_repository::PgMemberRepository;
use sentinel_api::adapters::outbound::postgres::system::bot_config_repository::PgBotConfigRepository;
use sentinel_api::application::casino::blackjack_service::BlackjackService;
use sentinel_api::application::casino::manage_wallet_service::ManageWalletService;
use sentinel_api::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use sentinel_api::ports::outbound::casino::wallet_repository::WalletRepository;
use sentinel_core::domain::entities::coude::taunt::TauntEvent;
use sentinel_core::domain::entities::coude::taunt::TauntsConfig;
use sentinel_core::domain::errors::DomainError;
use sqlx::PgPool;

async fn setup_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url)
        .await
        .expect("Impossible de se connecter a la base de test")
}

fn unique_guild() -> String {
    format!(
        "{}",
        uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    )
}

/// Taunts UC qui ne declenche jamais rien — suffit pour les tests d'integration
/// qui valident uniquement la conservation du solde / la mecanique de jeu.
struct NoopTauntsUc;
#[async_trait]
impl ManageCoudeTauntsUseCase for NoopTauntsUc {
    async fn on_player_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_player_lost(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_player_drew(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn on_player_stolen_from(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_player_defended_steal(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn on_bankruptcy(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_jackpot(
        &self,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_generous_donor(
        &self,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_bj_natural(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_bj_hand_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_bj_hand_bust(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn get_config(&self, _: &str) -> Result<TauntsConfig, DomainError> {
        unimplemented!()
    }
    async fn set_channel(&self, _: &str, _: Option<&str>) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_rename_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_messages_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_opt_out(&self, _: &str, _: &str, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn is_opted_out(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn list_opt_outs(&self, _: &str) -> Result<Vec<String>, DomainError> {
        Ok(vec![])
    }
}

fn build_service(pool: PgPool) -> BlackjackService {
    let bj_repo = Arc::new(PgBlackjackRepository::new(pool.clone()));
    let wallet_repo = Arc::new(PgWalletRepository::new(pool.clone()));
    let taunts_uc: Arc<dyn ManageCoudeTauntsUseCase> = Arc::new(NoopTauntsUc);
    let member_repo = Arc::new(PgMemberRepository::new(pool.clone()));
    let bot_config_repo = Arc::new(PgBotConfigRepository::new(pool.clone()));
    let wallet_uc = Arc::new(ManageWalletService::new(
        wallet_repo.clone(),
        taunts_uc,
        member_repo,
        bot_config_repo,
    ));
    BlackjackService::new(bj_repo, wallet_repo, wallet_uc)
}

// ══════════════════════════════════════════════════════════
//  Game lifecycle
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn blackjack_start_game_debits_wallet() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo
        .get_or_create(&gid, "player1", "Alice", 500)
        .await
        .unwrap();

    let svc = build_service(pool.clone());
    let game = svc
        .start_game(
            gid.clone().into(),
            "player1".into(),
            "Alice".into(),
            100,
            10,
            10000,
            500,
            1.5,
        )
        .await
        .unwrap()
        .game;
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
    wallet_repo
        .get_or_create(&gid, "player1", "Alice", 1000)
        .await
        .unwrap();

    let svc = build_service(pool.clone());
    let game1 = svc
        .start_game(
            gid.clone().into(),
            "player1".into(),
            "Alice".into(),
            50,
            10,
            10000,
            1000,
            1.5,
        )
        .await;
    assert!(game1.is_ok());

    // Si le premier jeu est encore en cours, le second doit echouer
    let game1 = game1.unwrap().game;
    if game1.status == "playing" {
        let game2 = svc
            .start_game(
                gid.clone().into(),
                "player1".into(),
                "Alice".into(),
                50,
                10,
                10000,
                1000,
                1.5,
            )
            .await;
        assert!(game2.is_err(), "Devrait refuser une deuxieme partie active");
    }
}

#[tokio::test]
async fn blackjack_hit_and_stand() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo
        .get_or_create(&gid, "player1", "Alice", 500)
        .await
        .unwrap();

    let svc = build_service(pool.clone());
    let game = svc
        .start_game(
            gid.clone().into(),
            "player1".into(),
            "Alice".into(),
            50,
            10,
            10000,
            500,
            1.5,
        )
        .await
        .unwrap()
        .game;

    if game.status != "playing" {
        return; // Blackjack naturel, pas de hit possible
    }

    // Hit
    let game = svc.hit(game.id).await.unwrap().game;
    assert!(game.player_hand.len() >= 3);

    if game.status == "playing" {
        // Stand
        let game = svc.stand(game.id).await.unwrap().game;
        assert_ne!(
            game.status, "playing",
            "Le jeu devrait etre termine apres stand"
        );
        assert!(game.dealer_score > 0);
    }
}

#[tokio::test]
async fn blackjack_wallet_balance_conserved() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo
        .get_or_create(&gid, "player1", "Alice", 500)
        .await
        .unwrap();

    let svc = build_service(pool.clone());
    let game = svc
        .start_game(
            gid.clone().into(),
            "player1".into(),
            "Alice".into(),
            100,
            10,
            10000,
            500,
            1.5,
        )
        .await
        .unwrap()
        .game;

    // Terminer la partie
    let final_game = if game.status == "playing" {
        svc.stand(game.id).await.unwrap().game
    } else {
        game
    };

    let wallet = wallet_repo.get(&gid, "player1").await.unwrap().unwrap();

    // Verifier la coherence : solde = 500 - mise + payout
    let expected = 500 - 100 + final_game.payout;
    assert_eq!(
        wallet.coins, expected,
        "Solde incoherent : 500 - 100 + {} = {} mais wallet = {}",
        final_game.payout, expected, wallet.coins
    );
}

// ── Double down ──

#[tokio::test]
async fn blackjack_double_down_debits_additional_bet() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo
        .get_or_create(&gid, "player1", "Alice", 1000)
        .await
        .unwrap();

    let svc = build_service(pool.clone());
    let game = svc
        .start_game(
            gid.clone().into(),
            "player1".into(),
            "Alice".into(),
            50,
            10,
            10000,
            1000,
            1.5,
        )
        .await
        .unwrap()
        .game;

    if game.status != "playing" || game.player_hand.len() != 2 {
        return; // Skip si blackjack naturel
    }

    let doubled = svc.double_down(game.id).await.unwrap().game;
    assert!(doubled.doubled);
    assert_eq!(doubled.bet, 100); // mise doublee
                                  // Le jeu doit etre termine (une carte tiree + dealer joue si pas bust)
    assert_ne!(doubled.status, "playing");

    // Wallet : -100 au total (50 + 50 double) + payout
    let wallet = wallet_repo.get(&gid, "player1").await.unwrap().unwrap();
    let expected = 1000 - 100 + doubled.payout;
    assert_eq!(wallet.coins, expected);
}

#[tokio::test]
async fn blackjack_double_down_rejected_after_hit() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo
        .get_or_create(&gid, "player1", "Alice", 1000)
        .await
        .unwrap();
    let svc = build_service(pool.clone());
    let game = svc
        .start_game(
            gid.clone().into(),
            "player1".into(),
            "Alice".into(),
            50,
            10,
            10000,
            1000,
            1.5,
        )
        .await
        .unwrap()
        .game;
    if game.status != "playing" {
        return;
    }
    let after_hit = svc.hit(game.id).await.unwrap().game;
    if after_hit.status == "playing" && after_hit.player_hand.len() > 2 {
        // double_down doit echouer : plus de 2 cartes
        let err = svc.double_down(after_hit.id).await.unwrap_err();
        assert!(matches!(err, DomainError::ValidationError(_)));
    }
}

// ── get_active / list_games / cancel_game ──

#[tokio::test]
async fn blackjack_get_active_returns_none_without_game() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let svc = build_service(pool.clone());
    let active = svc.get_active(&gid, "nobody").await.unwrap();
    assert!(active.is_none());
}

#[tokio::test]
async fn blackjack_get_active_returns_playing_game() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo
        .get_or_create(&gid, "p1", "Alice", 1000)
        .await
        .unwrap();
    let svc = build_service(pool.clone());
    let game = svc
        .start_game(
            gid.clone().into(),
            "p1".into(),
            "Alice".into(),
            50,
            10,
            10000,
            1000,
            1.5,
        )
        .await
        .unwrap()
        .game;
    if game.status != "playing" {
        return;
    }
    let active = svc.get_active(&gid, "p1").await.unwrap();
    assert!(active.is_some());
    assert_eq!(active.unwrap().id, game.id);
}

#[tokio::test]
async fn blackjack_list_games_returns_created_games() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo
        .get_or_create(&gid, "p1", "Alice", 1000)
        .await
        .unwrap();
    let svc = build_service(pool.clone());
    svc.start_game(
        gid.clone().into(),
        "p1".into(),
        "Alice".into(),
        50,
        10,
        10000,
        1000,
        1.5,
    )
    .await
    .unwrap();
    let games = svc.list_games(&gid, None).await.unwrap();
    assert!(!games.is_empty());
}

#[tokio::test]
async fn blackjack_cancel_game_refunds_wallet() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo
        .get_or_create(&gid, "p1", "Alice", 1000)
        .await
        .unwrap();
    let svc = build_service(pool.clone());
    let game = svc
        .start_game(
            gid.clone().into(),
            "p1".into(),
            "Alice".into(),
            50,
            10,
            10000,
            1000,
            1.5,
        )
        .await
        .unwrap()
        .game;
    // Si la partie n'est pas en cours (blackjack naturel), cancel peut echouer : skip.
    if game.status != "playing" {
        return;
    }
    svc.cancel_game(game.id).await.unwrap();
    // get_active doit revenir None
    let active = svc.get_active(&gid, "p1").await.unwrap();
    assert!(active.is_none());
}

#[tokio::test]
async fn blackjack_hit_not_found_returns_error() {
    let pool = setup_pool().await;
    let svc = build_service(pool);
    let err = svc.hit(uuid::Uuid::new_v4()).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn blackjack_stand_not_found_returns_error() {
    let pool = setup_pool().await;
    let svc = build_service(pool);
    let err = svc.stand(uuid::Uuid::new_v4()).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn blackjack_double_down_not_found_returns_error() {
    let pool = setup_pool().await;
    let svc = build_service(pool);
    let err = svc.double_down(uuid::Uuid::new_v4()).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn blackjack_start_game_rejects_bet_below_min() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let svc = build_service(pool);
    let err = svc
        .start_game(
            gid.into(),
            "p1".into(),
            "Alice".into(),
            5,
            10,
            10000,
            500,
            1.5,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn blackjack_start_game_rejects_bet_above_max() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let svc = build_service(pool);
    let err = svc
        .start_game(
            gid.into(),
            "p1".into(),
            "Alice".into(),
            99_999,
            10,
            10000,
            500,
            1.5,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn blackjack_start_game_conflict_when_game_already_active() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo
        .get_or_create(&gid, "p1", "Alice", 1000)
        .await
        .unwrap();
    let svc = build_service(pool);
    let game = svc
        .start_game(
            gid.clone().into(),
            "p1".into(),
            "Alice".into(),
            50,
            10,
            10000,
            1000,
            1.5,
        )
        .await
        .unwrap()
        .game;
    if game.status != "playing" {
        return;
    }
    let err = svc
        .start_game(
            gid.into(),
            "p1".into(),
            "Alice".into(),
            50,
            10,
            10000,
            1000,
            1.5,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::Conflict(_)));
}

#[tokio::test]
async fn blackjack_hit_rejects_when_game_not_playing() {
    // On force un bust pour mettre le status a "player_bust", puis on tente un hit → Conflict
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo
        .get_or_create(&gid, "p1", "Alice", 1000)
        .await
        .unwrap();
    let svc = build_service(pool);
    let mut game = svc
        .start_game(
            gid.clone().into(),
            "p1".into(),
            "Alice".into(),
            50,
            10,
            10000,
            1000,
            1.5,
        )
        .await
        .unwrap()
        .game;
    // On hit jusqu'a ce que le game termine
    let mut tries = 0;
    while game.status == "playing" && tries < 10 {
        game = svc.hit(game.id).await.unwrap().game;
        tries += 1;
    }
    if game.status == "playing" {
        return;
    } // safeguard
    let err = svc.hit(game.id).await.unwrap_err();
    assert!(matches!(err, DomainError::Conflict(_)));
}

#[tokio::test]
async fn blackjack_many_games_cover_various_outcomes() {
    // Joue 15 parties rapidement pour couvrir statistiquement les branches
    // resolve_game (player_win / dealer_win / push / dealer_bust).
    let pool = setup_pool().await;
    let gid = unique_guild();
    let wallet_repo = PgWalletRepository::new(pool.clone());
    wallet_repo
        .get_or_create(&gid, "p1", "Alice", 100_000)
        .await
        .unwrap();
    let svc = build_service(pool);
    for _ in 0..15 {
        let game = svc
            .start_game(
                gid.clone().into(),
                "p1".into(),
                "Alice".into(),
                50,
                10,
                10000,
                100_000,
                1.5,
            )
            .await
            .unwrap()
            .game;
        if game.status == "playing" {
            let _ = svc.stand(game.id).await.unwrap();
        }
    }
}
