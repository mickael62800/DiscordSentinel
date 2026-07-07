//! Port outbound pour les tentatives de vol (`coude_steal_attempts`, Phase 5).
//!
//! Persistance simple + transitions de statut atomiques ; aucune regle metier
//! ici (elle vit dans `ManageStealAttemptsUseCase`).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::steal::attempt::NewStealAttempt;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait StealAttemptRepository: Send + Sync {
    /// Insere une tentative au statut `pending`.
    async fn insert_pending(&self, attempt: &NewStealAttempt) -> Result<(), DomainError>;

    /// Transition atomique `pending -> defended`. Renvoie `true` si CET appel a
    /// fait la transition (sinon deja defended/expired/resolved ou id inconnu).
    async fn mark_defended(&self, id: Uuid) -> Result<bool, DomainError>;

    /// CLAIM atomique de la resolution : `pending|defended|expired -> resolved`.
    /// Renvoie `true` uniquement si CET appel a fait la transition finale.
    async fn claim_resolved(&self, id: Uuid) -> Result<bool, DomainError>;
}
