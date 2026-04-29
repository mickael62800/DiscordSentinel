//! Use case "TOUT-OU-RIEN" cote API (Phase 2 #1 audit).
//!
//! Migration RNG bot -> API : la decision pile/face est prise par le
//! serveur (auditable, rejouable). Le bot devient un appel API + une
//! animation client + l'affichage du verdict.
//!
//! Pipeline complet (cf. `PlayToutOuRienService`) :
//!  1. Verifier le cooldown weekly (`tout_ou_rien` action key).
//!  2. Lire le solde du joueur (>= `MIN_BALANCE_FOR_PLAY`).
//!  3. Tirer un f64 dans [0, 1) et resoudre via `resolve_outcome`.
//!  4. Calculer le delta via `coin_delta` (Win = +balance, Lose = -80%).
//!  5. Appliquer la mutation wallet (credit/debit selon outcome).
//!  6. Poser le cooldown (7j).
//!  7. Logger la tentative dans `tout_ou_rien_log` (Memorial des clodos).
//!  8. Retourner le `ToutOuRienResolution` au caller.
//!
//! La taille minimale (`MIN_BALANCE_FOR_PLAY`) reste cote API : le bot
//! affiche juste l'erreur retournee.

use async_trait::async_trait;

use crate::domain::entities::coude::tout_ou_rien::ToutOuRienOutcome;
use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::UserId;

/// Solde minimum requis pour declencher un tout-ou-rien (centralise ici
/// plutot que cote bot pour cloturer Phase 1 / 12 magic constants).
pub const MIN_BALANCE_FOR_PLAY: i64 = 100;

#[derive(Debug, Clone)]
pub struct PlayToutOuRienCommand {
    pub guild_id: String,
    pub user_id: UserId,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct ToutOuRienResolution {
    /// Solde au moment du tirage (mise effective).
    pub initial_coins: i64,
    pub outcome: ToutOuRienOutcome,
    /// `+initial_coins` si Win, `-(0.8 * initial_coins)` si Lose.
    pub delta: i64,
    /// Solde apres application du delta (clamp >= 0).
    pub final_balance: i64,
}

#[async_trait]
pub trait PlayToutOuRienUseCase: Send + Sync {
    /// Errors :
    /// - `RateLimited` si cooldown weekly encore actif.
    /// - `ValidationError("Solde insuffisant ...")` si `coins < MIN_BALANCE_FOR_PLAY`.
    /// - `Internal` sur erreur DB / wallet.
    async fn play(
        &self,
        cmd: PlayToutOuRienCommand,
    ) -> Result<ToutOuRienResolution, DomainError>;
}
