//! Use case du systeme de braquage (Phase 10).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sentinel_core::domain::entities::coude::heist::HeistOutcome;
use sentinel_core::domain::errors::DomainError;

/// Resultat du check cooldown : si `ready == false`, le joueur doit
/// attendre jusqu'a `next_attempt_at`.
#[derive(Debug, Clone)]
pub struct HeistCooldownStatus {
    pub ready: bool,
    pub next_attempt_at: Option<DateTime<Utc>>,
    pub last_success: Option<bool>,
}

/// Etat prison simplifie pour le RPC (pas d'infos internes).
#[derive(Debug, Clone)]
pub struct PrisonStatusInfo {
    pub in_prison: bool,
    pub released_at: Option<DateTime<Utc>>,
    pub reason: Option<String>,
}

#[async_trait]
pub trait ManageCoudeHeistUseCase: Send + Sync {
    /// Status du cooldown hebdo pour un joueur.
    async fn get_cooldown_status(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<HeistCooldownStatus, DomainError>;

    /// Status prison actuel. Si le `released_at` est passe, retourne
    /// `in_prison: false` meme si la row existe.
    async fn get_prison_status(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<PrisonStatusInfo, DomainError>;

    /// Tente un braquage. Echoue avec DomainError::Forbidden si :
    /// - le joueur est en prison
    /// - le cooldown hebdo n'est pas ecoule
    /// - la caisse est vide
    ///
    /// Sinon : calcule la chance, roll, consomme les items actifs
    /// dans l'inventaire (quel que soit le resultat), debite/credite
    /// la caisse + le joueur si succes, envoie en prison si echec.
    async fn attempt_heist(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<HeistOutcome, DomainError>;
}
