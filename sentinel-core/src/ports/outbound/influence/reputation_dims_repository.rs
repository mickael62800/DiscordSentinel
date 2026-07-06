//! Port outbound : reputation multi-dimensionnelle d'un citoyen.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::influence::reputation_dims::{ReputationDim, ReputationDims};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ReputationDimsRepository: Send + Sync {
    /// Dimensions d'un citoyen (toutes a 0 si aucune ligne).
    async fn get(&self, citizen_id: Uuid) -> Result<ReputationDims, DomainError>;

    /// Ajuste une dimension (upsert) et renvoie sa nouvelle valeur.
    async fn adjust(
        &self,
        citizen_id: Uuid,
        dim: ReputationDim,
        delta: i64,
    ) -> Result<i64, DomainError>;
}
