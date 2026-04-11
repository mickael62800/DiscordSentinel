use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::{BlackjackGame, calculate_score, create_deck};
use crate::domain::errors::DomainError;
use crate::ports::outbound::{BlackjackRepository, WalletRepository};

pub struct BlackjackService {
    repo: Arc<dyn BlackjackRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
}

impl BlackjackService {
    pub fn new(
        repo: Arc<dyn BlackjackRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
    ) -> Self {
        Self { repo, wallet_repo }
    }

    /// Démarre une nouvelle partie de blackjack.
    /// `blackjack_payout` = multiplicateur du gain pour un blackjack naturel (defaut 1.5 → payout total = mise * 2.5).
    pub async fn start_game(
        &self,
        guild_id: String,
        user_id: String,
        username: String,
        bet: i64,
        min_bet: i64,
        max_bet: i64,
        starting_coins: i64,
        blackjack_payout: f64,
    ) -> Result<BlackjackGame, DomainError> {
        // Validation de la mise
        if bet < min_bet {
            return Err(DomainError::ValidationError(
                format!("La mise minimum est de {} coins.", min_bet),
            ));
        }
        if bet > max_bet {
            return Err(DomainError::ValidationError(
                format!("La mise maximum est de {} coins.", max_bet),
            ));
        }

        // Vérifier qu'il n'y a pas de partie active
        if self.repo.get_active(&guild_id, &user_id).await?.is_some() {
            return Err(DomainError::Conflict(
                "Tu as déjà une partie de blackjack en cours.".into(),
            ));
        }

        // S'assurer que le wallet existe et débiter la mise
        self.wallet_repo
            .get_or_create(&guild_id, &user_id, &username, starting_coins)
            .await?;
        self.wallet_repo
            .debit(&guild_id, &user_id, bet, "blackjack", "Mise blackjack")
            .await?;

        // Créer le deck et distribuer les cartes
        let mut deck = create_deck();
        let player_hand = vec![deck.pop().unwrap(), deck.pop().unwrap()];
        let dealer_hand = vec![deck.pop().unwrap(), deck.pop().unwrap()];

        let player_score = calculate_score(&player_hand);
        let dealer_score = calculate_score(&dealer_hand);

        // Vérifier le blackjack naturel
        let (status, payout, finished_at) = if player_score == 21 {
            // Blackjack naturel du joueur
            let payout = (bet as f64 * (1.0 + blackjack_payout)) as i64;
            ("player_blackjack".to_string(), payout, Some(Utc::now()))
        } else {
            ("playing".to_string(), 0, None)
        };

        // Si blackjack, créditer le gain
        if status == "player_blackjack" {
            self.wallet_repo
                .credit(&guild_id, &user_id, payout, "blackjack", "Blackjack ! Gain x2.5")
                .await?;
        }

        let game = BlackjackGame {
            id: Uuid::new_v4(),
            guild_id,
            user_id,
            username,
            bet,
            player_hand,
            dealer_hand,
            deck,
            status,
            player_score,
            dealer_score,
            doubled: false,
            payout,
            created_at: Utc::now(),
            finished_at,
        };

        self.repo.create(&game).await?;
        Ok(game)
    }

    /// Le joueur tire une carte supplémentaire.
    pub async fn hit(&self, game_id: Uuid) -> Result<BlackjackGame, DomainError> {
        let mut game = self.get_game(game_id).await?;
        self.ensure_playing(&game)?;

        // Tirer une carte
        let card = game.deck.pop().ok_or_else(|| {
            DomainError::Internal("Le deck est vide.".into())
        })?;
        game.player_hand.push(card);
        game.player_score = calculate_score(&game.player_hand);

        if game.player_score > 21 {
            // Bust
            game.status = "player_bust".to_string();
            game.payout = 0;
            game.finished_at = Some(Utc::now());
            game.dealer_score = calculate_score(&game.dealer_hand);
        }

        self.repo.update(&game).await?;
        Ok(game)
    }

    /// Le joueur reste avec sa main actuelle. Le dealer joue.
    pub async fn stand(&self, game_id: Uuid) -> Result<BlackjackGame, DomainError> {
        let mut game = self.get_game(game_id).await?;
        self.ensure_playing(&game)?;

        self.dealer_play(&mut game);
        self.resolve_game(&mut game).await?;

        self.repo.update(&game).await?;
        Ok(game)
    }

    /// Double down : doubler la mise, tirer une carte, puis le dealer joue.
    pub async fn double_down(&self, game_id: Uuid) -> Result<BlackjackGame, DomainError> {
        let mut game = self.get_game(game_id).await?;
        self.ensure_playing(&game)?;

        // On ne peut doubler qu'avec 2 cartes
        if game.player_hand.len() != 2 {
            return Err(DomainError::ValidationError(
                "Tu ne peux doubler qu'avec tes 2 premières cartes.".into(),
            ));
        }

        // Débiter la mise supplémentaire
        self.wallet_repo
            .debit(
                &game.guild_id,
                &game.user_id,
                game.bet,
                "blackjack",
                "Double down blackjack",
            )
            .await?;

        game.bet *= 2;
        game.doubled = true;

        // Tirer exactement une carte
        let card = game.deck.pop().ok_or_else(|| {
            DomainError::Internal("Le deck est vide.".into())
        })?;
        game.player_hand.push(card);
        game.player_score = calculate_score(&game.player_hand);

        if game.player_score > 21 {
            game.status = "player_bust".to_string();
            game.payout = 0;
            game.finished_at = Some(Utc::now());
            game.dealer_score = calculate_score(&game.dealer_hand);
        } else {
            // Le dealer joue
            self.dealer_play(&mut game);
            self.resolve_game(&mut game).await?;
        }

        self.repo.update(&game).await?;
        Ok(game)
    }

    /// Récupère la partie active d'un joueur.
    pub async fn get_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<BlackjackGame>, DomainError> {
        self.repo.get_active(guild_id, user_id).await
    }

    // ── Helpers internes ──

    async fn get_game(&self, game_id: Uuid) -> Result<BlackjackGame, DomainError> {
        self.repo
            .get_by_id(game_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Partie de blackjack introuvable.".into()))
    }

    fn ensure_playing(&self, game: &BlackjackGame) -> Result<(), DomainError> {
        if game.status != "playing" {
            return Err(DomainError::Conflict(
                "Cette partie n'est plus en cours.".into(),
            ));
        }
        Ok(())
    }

    /// Le dealer tire jusqu'à atteindre 17 ou plus.
    fn dealer_play(&self, game: &mut BlackjackGame) {
        game.dealer_score = calculate_score(&game.dealer_hand);
        while game.dealer_score < 17 {
            if let Some(card) = game.deck.pop() {
                game.dealer_hand.push(card);
                game.dealer_score = calculate_score(&game.dealer_hand);
            } else {
                break;
            }
        }
    }

    /// Résout la partie après que le dealer ait joué : détermine le statut et crédite le wallet.
    async fn resolve_game(&self, game: &mut BlackjackGame) -> Result<(), DomainError> {
        game.finished_at = Some(Utc::now());

        if game.dealer_score > 21 {
            game.status = "dealer_bust".to_string();
            game.payout = game.bet * 2;
        } else if game.player_score > game.dealer_score {
            game.status = "player_win".to_string();
            game.payout = game.bet * 2;
        } else if game.player_score < game.dealer_score {
            game.status = "dealer_win".to_string();
            game.payout = 0;
        } else {
            // Égalité (push)
            game.status = "push".to_string();
            game.payout = game.bet;
        }

        // Créditer le wallet si gain ou push
        if game.payout > 0 {
            let description = match game.status.as_str() {
                "dealer_bust" => "Victoire blackjack (dealer bust)",
                "player_win" => "Victoire blackjack",
                "push" => "Égalité blackjack (mise remboursée)",
                _ => "Gain blackjack",
            };
            self.wallet_repo
                .credit(
                    &game.guild_id,
                    &game.user_id,
                    game.payout,
                    "blackjack",
                    description,
                )
                .await?;
        }

        Ok(())
    }
}
