//! Port outbound : registre des mouvements de capitaux (append-only).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::influence::capital::Capital;
use crate::domain::entities::influence::movement::CapitalMovement;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait MovementRepository: Send + Sync {
    /// Enregistre une variation de capital.
    async fn record(
        &self,
        guild_id: &str,
        citizen_id: Uuid,
        capital: Capital,
        delta: i64,
        reason: &str,
    ) -> Result<(), DomainError>;

    /// Derniers mouvements d'un citoyen (les plus recents d'abord).
    async fn list_recent(
        &self,
        citizen_id: Uuid,
        limit: i64,
    ) -> Result<Vec<CapitalMovement>, DomainError>;
}
