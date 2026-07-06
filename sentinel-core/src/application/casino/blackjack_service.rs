use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::casino::blackjack::calculate_score;
use crate::domain::entities::casino::blackjack::create_deck;
use crate::domain::entities::casino::blackjack::natural_deal_outcome;
use crate::domain::entities::casino::blackjack::BlackjackGame;
use crate::domain::entities::casino::blackjack::NaturalDealOutcome;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::outbound::casino::blackjack_repository::BlackjackRepository;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

/// Nom du module de config pour les reglages editables par serveur.
const MODULE_BOT_NAME: &str = "blackjack-bot";

/// Seuil de tirage du dealer par defaut : il tire tant que son score est
/// strictement inferieur a 17 (regle standard du blackjack).
pub const DEFAULT_DEALER_HIT_THRESHOLD: i32 = 17;

/// Borne le seuil de tirage du dealer a un intervalle jouable/non-exploitable.
/// En dehors de 16..=20, la regle est absurde (dealer qui ne tire jamais /
/// tire toujours) et casse l equilibre du jeu.
pub fn clamp_dealer_threshold(v: i32) -> i32 {
    v.clamp(16, 20)
}
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
    bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl BlackjackService {
    pub fn new(
        repo: Arc<dyn BlackjackRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
        wallet_uc: Arc<dyn ManageWalletUseCase>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self {
            repo,
            wallet_repo,
            wallet_uc,
            bot_config_repo,
        }
    }

    /// Charge le seuil de tirage du dealer (`dealer_hit_threshold`) depuis la
    /// config `blackjack-bot` de la guild. Defaut 17, clampe a 16..=20.
    /// Le domaine reste pur : la valeur (data) est passee a `dealer_play`.
    async fn load_dealer_threshold(&self, guild_id: &str) -> i32 {
        let cfg = self
            .bot_config_repo
            .get_config(guild_id, MODULE_BOT_NAME)
            .await
            .unwrap_or_default();
        let raw = cfg
            .iter()
            .find(|c| c.config_key == "dealer_hit_threshold")
            .and_then(|c| c.config_value.parse::<i32>().ok())
            .unwrap_or(DEFAULT_DEALER_HIT_THRESHOLD);
        clamp_dealer_threshold(raw)
    }

    /// Démarre une nouvelle partie de blackjack.
    /// `blackjack_payout` = multiplicateur du gain pour un blackjack naturel (defaut 1.5 → payout total = mise * 2.5).
    pub async fn start_game(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        username: String,
        bet: i64,
        min_bet: i64,
        max_bet: i64,
        starting_coins: i64,
        blackjack_payout: f64,
    ) -> Result<BlackjackActionResult, DomainError> {
        // Validation de la mise
        if bet < min_bet {
            return Err(DomainError::ValidationError(format!(
                "La mise minimum est de {} coins.",
                min_bet
            )));
        }
        if bet > max_bet {
            return Err(DomainError::ValidationError(format!(
                "La mise maximum est de {} coins.",
                max_bet
            )));
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

        // Vérifier le blackjack naturel (règle pure dans le domaine).
        let (status, payout, finished_at) = match natural_deal_outcome(player_score, dealer_score) {
            NaturalDealOutcome::Push => {
                // Double blackjack naturel (joueur ET croupier) : égalité.
                // La mise a déjà été débitée plus haut, on la rembourse à
                // l'identique -> gain net nul.
                ("push".to_string(), bet, Some(Utc::now()))
            }
            NaturalDealOutcome::PlayerBlackjack => {
                // Blackjack naturel : on TRONQUE (`.floor()`) au lieu d'arrondir.
                // `.round()` arrondissait le `.5` d'une mise impaire TOUJOURS vers
                // le haut -> +0,5 coin cree ex nihilo a chaque BJ (biais
                // inflationniste sur le wallet partage). floor => la maison ne
                // perd jamais, aucune creation de monnaie.
                let payout = (bet as f64 * (1.0 + blackjack_payout)).floor() as i64;
                ("player_blackjack".to_string(), payout, Some(Utc::now()))
            }
            NaturalDealOutcome::KeepPlaying => ("playing".to_string(), 0, None),
        };

        // Si blackjack ou push, créditer le gain/remboursement via wallet_uc
        // (jackpot auto-detecte).
        if (status == "player_blackjack" || status == "push") && payout > 0 {
            let reason = if status == "push" {
                "Égalité blackjack (mise remboursée)"
            } else {
                "Blackjack ! Gain x2.5"
            };
            let credit_mut = self
                .wallet_uc
                .credit(&guild_id, &user_id, payout, "blackjack", reason)
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
        let wallet_balance = self
            .wallet_uc
            .get_balance(&game.guild_id, &game.user_id)
            .await?;
        Ok(BlackjackActionResult {
            game,
            taunt_events,
            wallet_balance,
        })
    }

    /// Le joueur tire une carte supplémentaire.
    pub async fn hit(&self, game_id: Uuid) -> Result<BlackjackActionResult, DomainError> {
        let mut game = self.get_game(game_id).await?;
        self.ensure_playing(&game)?;

        // Tirer une carte
        let card = game
            .deck
            .pop()
            .ok_or_else(|| DomainError::Internal("Le deck est vide.".into()))?;
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
        let wallet_balance = self
            .wallet_uc
            .get_balance(&game.guild_id, &game.user_id)
            .await?;
        Ok(BlackjackActionResult {
            game,
            taunt_events: vec![],
            wallet_balance,
        })
    }

    /// Le joueur reste avec sa main actuelle. Le dealer joue.
    pub async fn stand(&self, game_id: Uuid) -> Result<BlackjackActionResult, DomainError> {
        let mut game = self.get_game(game_id).await?;
        self.ensure_playing(&game)?;

        let hit_threshold = self.load_dealer_threshold(&game.guild_id).await;
        self.dealer_play(&mut game, hit_threshold);
        // Résolution PURE (status/payout en mémoire, pas de crédit).
        self.apply_resolution(&mut game);

        // Compare-and-set : seul l'appel qui flippe `playing -> terminé`
        // gagne. Un `stand` concurrent reçoit Conflict ICI et ne crédite
        // jamais (anti double-payout #4). Le crédit vient APRÈS l'update.
        self.repo.update(&game).await?;
        let taunt_events = self.credit_payout(&game).await?;

        let wallet_balance = self
            .wallet_uc
            .get_balance(&game.guild_id, &game.user_id)
            .await?;
        Ok(BlackjackActionResult {
            game,
            taunt_events,
            wallet_balance,
        })
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

        // Mise supplémentaire à débiter (= mise initiale, avant doublement).
        let extra_bet = game.bet;

        // Pré-check best-effort du solde : évite de consommer la partie sur un
        // solde insuffisant. Le débit réel post-CAS reste la garantie dure
        // (validation wallet_uc.debit + CHECK coins>=0 en DB).
        let balance = self
            .wallet_uc
            .get_balance(&game.guild_id, &game.user_id)
            .await?;
        if balance < extra_bet {
            return Err(DomainError::ValidationError(
                "Solde insuffisant pour doubler.".into(),
            ));
        }

        // Construire l'état final EN MÉMOIRE — aucune mutation wallet ici.
        game.bet = game.bet.saturating_mul(2);
        game.doubled = true;

        // Tirer exactement une carte
        let card = game
            .deck
            .pop()
            .ok_or_else(|| DomainError::Internal("Le deck est vide.".into()))?;
        game.player_hand.push(card);
        game.player_score = calculate_score(&game.player_hand);

        if game.player_score > 21 {
            game.status = "player_bust".to_string();
            game.payout = 0;
            game.finished_at = Some(Utc::now());
            game.dealer_score = calculate_score(&game.dealer_hand);
        } else {
            // Le dealer joue, puis résolution PURE (pas de crédit).
            let hit_threshold = self.load_dealer_threshold(&game.guild_id).await;
            self.dealer_play(&mut game, hit_threshold);
            self.apply_resolution(&mut game);
        }

        // Compare-and-set : claim de la transition `playing -> terminé` AVANT
        // toute mutation wallet. Un double_down concurrent reçoit Conflict ICI
        // et ne débite/crédite jamais (anti double-débit + double-payout #4).
        self.repo.update(&game).await?;

        // Transition remportée : débiter la mise supplémentaire PUIS créditer
        // le payout. La partie ne peut plus repasser 'playing', donc aucun
        // autre appel ne rejouera ces mutations.
        let mut taunt_events: Vec<TauntEvent> = Vec::new();
        let debit_mut = self
            .wallet_uc
            .debit(
                &game.guild_id,
                &game.user_id,
                extra_bet,
                "blackjack",
                "Double down blackjack",
            )
            .await?;
        taunt_events.extend(debit_mut.triggered_taunts);
        let credit_taunts = self.credit_payout(&game).await?;
        taunt_events.extend(credit_taunts);

        let wallet_balance = self
            .wallet_uc
            .get_balance(&game.guild_id, &game.user_id)
            .await?;
        Ok(BlackjackActionResult {
            game,
            taunt_events,
            wallet_balance,
        })
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

    /// Le dealer tire jusqu'à atteindre `hit_threshold` ou plus (regle pure :
    /// le seuil est fourni en parametre, deja clampe par l appelant).
    fn dealer_play(&self, game: &mut BlackjackGame, hit_threshold: i32) {
        game.dealer_score = calculate_score(&game.dealer_hand);
        while game.dealer_score < hit_threshold {
            if let Some(card) = game.deck.pop() {
                game.dealer_hand.push(card);
                game.dealer_score = calculate_score(&game.dealer_hand);
            } else {
                break;
            }
        }
    }

    /// Résout la partie après que le dealer ait joué : détermine le statut
    /// et le payout **en mémoire uniquement** (règle pure, AUCUNE mutation
    /// wallet). Le crédit est fait séparément par `credit_payout`, APRÈS que
    /// la transition d'état ait été remportée (compare-and-set repo.update),
    /// pour éviter le double-payout sur deux actions concurrentes (#4).
    fn apply_resolution(&self, game: &mut BlackjackGame) {
        game.finished_at = Some(Utc::now());

        if game.dealer_score > 21 {
            game.status = "dealer_bust".to_string();
            game.payout = game.bet.saturating_mul(2);
        } else if game.player_score > game.dealer_score {
            game.status = "player_win".to_string();
            game.payout = game.bet.saturating_mul(2);
        } else if game.player_score < game.dealer_score {
            game.status = "dealer_win".to_string();
            game.payout = 0;
        } else {
            // Égalité (push)
            game.status = "push".to_string();
            game.payout = game.bet;
        }
    }

    /// Crédite le wallet pour un payout déjà calculé et déjà persisté (le
    /// `repo.update` guardé a réussi, donc CET appel est le seul à avoir
    /// remporté la transition `playing -> terminé`). À n'appeler QU'APRÈS un
    /// `repo.update` réussi : si l'update retourne Conflict (0 rows), un autre
    /// appel concurrent a déjà résolu la partie et crédité — ne PAS recréditer.
    async fn credit_payout(&self, game: &BlackjackGame) -> Result<Vec<TauntEvent>, DomainError> {
        if game.payout <= 0 {
            return Ok(vec![]);
        }
        let description = match game.status.as_str() {
            "dealer_bust" => "Victoire blackjack (dealer bust)",
            "player_win" => "Victoire blackjack",
            "push" => "Égalité blackjack (mise remboursée)",
            "player_blackjack" => "Blackjack ! Gain x2.5",
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
        Ok(credit_mut.triggered_taunts)
    }
}

#[cfg(test)]
#[path = "tests/blackjack.rs"]
mod tests;
