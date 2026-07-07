//! Port inbound (use case) pour les tentatives de vol (`coude_steal_attempts`).
//!
//! Le handler HTTP ne fait que parser/mapper : le calcul de la fenetre de
//! defense (`expires_at`) et les transitions de statut vivent dans le service.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::steal::attempt::CreatedStealAttempt;
use crate::domain::errors::DomainError;

/// Commande de creation d'une tentative de vol (issue du /voler du bot).
#[derive(Debug, Clone)]
pub struct CreateStealAttempt {
    pub guild_id: String,
    pub thief_id: String,
    pub target_id: String,
    pub message_id: String,
    pub channel_id: String,
    /// Duree de la fenetre de defense en secondes (le bot envoie 60).
    pub window_secs: i64,
}

#[async_trait]
pub trait ManageStealAttemptsUseCase: Send + Sync {
    /// Cree une tentative `pending` : genere l'id et calcule `expires_at`.
    async fn create_attempt(
        &self,
        cmd: CreateStealAttempt,
    ) -> Result<CreatedStealAttempt, DomainError>;

    /// Marque la tentative comme defendue (idempotent cote appelant).
    async fn mark_defended(&self, id: Uuid) -> Result<(), DomainError>;

    /// Claim atomique de la resolution : `true` si CET appel a resolu.
    async fn claim_resolved(&self, id: Uuid) -> Result<bool, DomainError>;
}
