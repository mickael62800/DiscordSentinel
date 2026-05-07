//! Port outbound pour le systeme de braquage (Phase 10).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sentinel_core::domain::entities::coude::heist::HeistAttempt;
use sentinel_core::domain::entities::coude::heist::PrisonState;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait HeistRepository: Send + Sync {
    /// Derniere tentative de braquage d'un joueur (pour le check cooldown).
    /// Retourne None si le joueur n'a jamais braque.
    async fn last_attempt(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<HeistAttempt>, DomainError>;

    /// Insere une nouvelle tentative (succes ou echec) dans le log.
    async fn record_attempt(
        &self,
        guild_id: &str,
        user_id: &str,
        success: bool,
        amount_stolen: i64,
        chance_percent: i32,
        tools_used: &[String],
    ) -> Result<HeistAttempt, DomainError>;

    /// Etat de prison actuel du joueur (None = jamais incarcere ou
    /// deja libere). La methode NE filtre PAS `released_at > NOW` :
    /// le caller decide.
    async fn get_prison(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<PrisonState>, DomainError>;

    /// Envoie un joueur en prison (upsert). `released_at` = absolu.
    async fn send_to_prison(
        &self,
        guild_id: &str,
        user_id: &str,
        released_at: DateTime<Utc>,
        reason: &str,
    ) -> Result<(), DomainError>;
}
