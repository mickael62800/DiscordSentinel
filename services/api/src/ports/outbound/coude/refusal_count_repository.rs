//! Port outbound pour les compteurs de refus (cf. COUPE_AMELIORATIONS 5.3
//! — dette d honneur).

use async_trait::async_trait;

use crate::domain::entities::coude::refusal_count::RefusalCount;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait CoudeRefusalCountRepository: Send + Sync {
    /// Incremente le compteur (UPSERT). Retourne le nouveau count.
    async fn increment(
        &self,
        guild_id: &str,
        requester_id: &str,
        refuser_id: &str,
    ) -> Result<i32, DomainError>;

    /// Lit le compteur (None si jamais refuse).
    async fn get(
        &self,
        guild_id: &str,
        requester_id: &str,
        refuser_id: &str,
    ) -> Result<Option<RefusalCount>, DomainError>;

    /// Reset le compteur a 0 (utilise apres /honneur invoque).
    async fn reset(
        &self,
        guild_id: &str,
        requester_id: &str,
        refuser_id: &str,
    ) -> Result<(), DomainError>;
}
