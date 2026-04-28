//! Port outbound pour les primes collectives (cf. COUPE_AMELIORATIONS 5.3).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::ActiveBounty;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait CoudeBountyRepository: Send + Sync {
    /// Cree une nouvelle prime ouverte avec un montant initial. Echoue
    /// avec Conflict si une prime ouverte existe deja sur cette cible.
    async fn open(
        &self,
        guild_id: &str,
        target_id: &str,
        initial_amount: i64,
    ) -> Result<Uuid, DomainError>;

    /// Recupere la prime ouverte sur une cible (None si aucune).
    async fn get_open(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveBounty>, DomainError>;

    /// Ajoute un montant a la prime + log la contribution. Atomic.
    /// No-op si la prime n est pas ouverte.
    async fn contribute(
        &self,
        bounty_id: Uuid,
        contributor_id: &str,
        contributor_name: &str,
        amount: i64,
    ) -> Result<i64, DomainError>;

    /// Marque la prime comme claimed. Retourne le montant total empoche.
    /// Echoue avec NotFound si la prime n existe pas, Conflict si deja
    /// claimed.
    async fn claim(
        &self,
        bounty_id: Uuid,
        claimer_id: &str,
    ) -> Result<i64, DomainError>;
}
