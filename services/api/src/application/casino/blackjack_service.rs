use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::{BlackjackGame, TauntEvent, calculate_score, create_deck};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_wallet::ManageWalletUseCase;
use crate::ports::outbound::{BlackjackRepository, WalletRepository};

/// Resultat d'une action de jeu : la partie mise a jour + la liste des
/// `TauntEvent` declenches par les mutations wallet (faillite, jackpot).
/// La couche transport (gRPC / HTTP) est responsable de propager ces
/// taunts vers le bot, qui les dispatchera.
#[derive(Debug, Clone)]
pub struct BlackjackActionResult {
    pub game: BlackjackGame,
    pub taunt_events: Vec<TauntEvent>,
    /// Solde du wallet apres l action (pour affichage live dans l embed).
    pub wallet_balance: i64,
}

/// # Migration wallet unifie (Migration #4)
///
/// Les mutations `user_wallets` passent maintenant par `wallet_uc` :
/// - `start_game` : debit de la mise + credit si blackjack naturel
/// - `double_down` : debit supplementaire + credit si gain
/// - `stand` / `hit` (bust/resolve) : credit du payout sur victoire
///
/// Le use case wallet detecte automatiquement faillite / jackpot et
/// retourne les `TauntEvent` associes, qui sont concatenes dans le
/// `BlackjackActionResult`. Les taunts specifiques blackjack
/// (BjNatural21 / BjWinStreak / BjBustStreak) restent cables a la main
/// cote bot via les endpoints `track_bj_*`.
///
/// `wallet_repo` est conserve pour `get_or_create` au demarrage de la
/// toute premiere partie (le wallet_uc ne l'expose pas).
///
/// `cancel_game` (admin) n'est PAS migre : il utilise sa propre tx
/// composite dans le repo blackjack. Skippe pour garder le scope de la
/// migration ciblee sur les flows joueurs (start/hit/stand/double).
pub struct BlackjackService {
    repo: Arc<dyn BlackjackRepository>,
    wallet_repo: Arc<dyn WalletRepository>,
    wallet_uc: Arc<dyn ManageWalletUseCase>,
}

impl BlackjackService {
    pub fn new(
        repo: Arc<dyn BlackjackRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
        wallet_uc: Arc<dyn ManageWalletUseCase>,
    ) -> Self {
        Self { repo, wallet_repo, wallet_uc }
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
    ) -> Result<BlackjackActionResult, DomainError> {
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

        // S'assurer que le wallet existe (le wallet_uc suppose qu'il existe).
        self.wallet_repo
            .get_or_create(&guild_id, &user_id, &username, starting_coins)
            .await?;

        let mut taunt_events: Vec<TauntEvent> = Vec::new();

        // Débiter la mise via le wallet UC centralise. Faillite auto-detectee.
        let debit_mut = self
            .wallet_uc
            .debit(&guild_id, &user_id, bet, "blackjack", "Mise blackjack")
            .await?;
        taunt_events.extend(debit_mut.triggered_taunts);

        // Créer le deck et distribuer les cartes
        let mut deck = create_deck();
        let err = || DomainError::Internal("Deck vide a la distribution".into());
        let player_hand = vec![deck.pop().ok_or_else(err)?, deck.pop().ok_or_else(err)?];
        let dealer_hand = vec![deck.pop().ok_or_else(err)?, deck.pop().ok_or_else(err)?];

        let player_score = calculate_score(&player_hand);
        let dealer_score = calculate_score(&dealer_hand);

        // Vérifier le blackjack naturel
        let (status, payout, finished_at) = if player_score == 21 {
            // Blackjack naturel du joueur. On `.round()` au lieu de
            // tronquer : pour bet=51 avec payout=1.5, le calcul donne
            // 127.5 -> 128 (fair) au lieu de 127 (perte de 0.5 coin).
            let payout = (bet as f64 * (1.0 + blackjack_payout)).round() as i64;
            ("player_blackjack".to_string(), payout, Some(Utc::now()))
        } else {
            ("playing".to_string(), 0, None)
        };

        // Si blackjack, créditer le gain via wallet_uc (jackpot auto-detecte).
        if status == "player_blackjack" && payout > 0 {
            let credit_mut = self
                .wallet_uc
                .credit(&guild_id, &user_id, payout, "blackjack", "Blackjack ! Gain x2.5")
                .await?;
            taunt_events.extend(credit_mut.triggered_taunts);
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
        let wallet_balance = self.wallet_uc.get_balance(&game.guild_id, &game.user_id).await?;
        Ok(BlackjackActionResult { game, taunt_events, wallet_balance })
    }

    /// Le joueur tire une carte supplémentaire.
    pub async fn hit(&self, game_id: Uuid) -> Result<BlackjackActionResult, DomainError> {
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
        // Hit ne touche pas le wallet (sauf si bust — pas de credit). Aucun
        // taunt wallet a propager ici.
        let wallet_balance = self.wallet_uc.get_balance(&game.guild_id, &game.user_id).await?;
        Ok(BlackjackActionResult { game, taunt_events: vec![], wallet_balance })
    }

    /// Le joueur reste avec sa main actuelle. Le dealer joue.
    pub async fn stand(&self, game_id: Uuid) -> Result<BlackjackActionResult, DomainError> {
        let mut game = self.get_game(game_id).await?;
        self.ensure_playing(&game)?;

        self.dealer_play(&mut game);
        let taunt_events = self.resolve_game(&mut game).await?;

        self.repo.update(&game).await?;
        let wallet_balance = self.wallet_uc.get_balance(&game.guild_id, &game.user_id).await?;
        Ok(BlackjackActionResult { game, taunt_events, wallet_balance })
    }

    /// Double down : doubler la mise, tirer une carte, puis le dealer joue.
    pub async fn double_down(&self, game_id: Uuid) -> Result<BlackjackActionResult, DomainError> {
        let mut game = self.get_game(game_id).await?;
        self.ensure_playing(&game)?;

        // On ne peut doubler qu'avec 2 cartes
        if game.player_hand.len() != 2 {
            return Err(DomainError::ValidationError(
                "Tu ne peux doubler qu'avec tes 2 premières cartes.".into(),
            ));
        }

        let mut taunt_events: Vec<TauntEvent> = Vec::new();

        // Débiter la mise supplémentaire via wallet_uc.
        let debit_mut = self
            .wallet_uc
            .debit(
                &game.guild_id,
                &game.user_id,
                game.bet,
                "blackjack",
                "Double down blackjack",
            )
            .await?;
        taunt_events.extend(debit_mut.triggered_taunts);

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
            let resolve_taunts = self.resolve_game(&mut game).await?;
            taunt_events.extend(resolve_taunts);
        }

        self.repo.update(&game).await?;
        let wallet_balance = self.wallet_uc.get_balance(&game.guild_id, &game.user_id).await?;
        Ok(BlackjackActionResult { game, taunt_events, wallet_balance })
    }

    /// Récupère la partie active d'un joueur.
    pub async fn get_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<BlackjackGame>, DomainError> {
        self.repo.get_active(guild_id, user_id).await
    }

    /// Liste les parties (optionnellement filtrees par status) — admin desktop.
    pub async fn list_games(
        &self,
        guild_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<BlackjackGame>, DomainError> {
        self.repo.list_by_guild(guild_id, status).await
    }

    /// Annule une partie en cours + rembourse la mise — admin desktop.
    ///
    /// Non migre vers wallet_uc : le refund est fait dans une tx composite
    /// dans le repo blackjack (select partie FOR UPDATE + update status +
    /// credit wallet + audit) pour garantir l'atomicite. Migrer
    /// impliquerait de casser cette tx ou d'utiliser `credit_tx`, ce qui
    /// depasse le scope de la migration #4 (flux joueurs uniquement).
    /// Admin-only : pas de taunt attendu sur un refund.
    pub async fn cancel_game(&self, id: Uuid) -> Result<(), DomainError> {
        self.repo.cancel_game(id).await
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

    /// Résout la partie après que le dealer ait joué : détermine le statut
    /// et crédite le wallet via wallet_uc. Retourne les TauntEvent eventuels.
    async fn resolve_game(&self, game: &mut BlackjackGame) -> Result<Vec<TauntEvent>, DomainError> {
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

        // Créditer le wallet si gain ou push (via wallet_uc -> jackpot auto).
        if game.payout > 0 {
            let description = match game.status.as_str() {
                "dealer_bust" => "Victoire blackjack (dealer bust)",
                "player_win" => "Victoire blackjack",
                "push" => "Égalité blackjack (mise remboursée)",
                _ => "Gain blackjack",
            };
            let credit_mut = self
                .wallet_uc
                .credit(
                    &game.guild_id,
                    &game.user_id,
                    game.payout,
                    "blackjack",
                    description,
                )
                .await?;
            return Ok(credit_mut.triggered_taunts);
        }

        Ok(vec![])
    }
}


#[cfg(test)]
#[path = "tests/blackjack.rs"]
mod tests;
