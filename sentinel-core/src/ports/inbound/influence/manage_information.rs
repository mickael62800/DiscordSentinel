//! Use case : information & medias (enquetes, intel, revelation/scandale).

use async_trait::async_trait;

use crate::domain::entities::influence::information::{Information, Investigation};
use crate::domain::errors::DomainError;

/// Resultat d'une enquete resolue (pour notification).
#[derive(Debug, Clone)]
pub struct ResolvedInvestigation {
    pub initiator_user_id: String,
    pub target_username: String,
    pub subject: String,
    pub success: bool,
}

/// Consequence d'une revelation (scandale).
#[derive(Debug, Clone)]
pub struct RevealOutcome {
    pub content: String,
    pub target_user_id: String,
    pub target_username: String,
    pub reputation_loss: i64,
    pub new_target_reputation: Option<i64>,
}

#[async_trait]
pub trait ManageInformationUseCase: Send + Sync {
    /// Ouvre une enquete sur une cible (debite l'Argent, resolution differee).
    async fn open_investigation(
        &self,
        guild_id: &str,
        initiator_user_id: &str,
        initiator_username: &str,
        target_user_id: &str,
        target_username: &str,
        subject: &str,
    ) -> Result<Investigation, DomainError>;

    /// Intel secret detenu par un citoyen.
    async fn list_intel(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<Vec<Information>, DomainError>;

    /// Revele une information : scandale + perte de reputation de la cible.
    async fn reveal(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        info_id: &str,
    ) -> Result<RevealOutcome, DomainError>;

    /// Resout les enquetes echues (worker). Renvoie les resultats a notifier.
    async fn resolve_due(&self) -> Result<Vec<ResolvedInvestigation>, DomainError>;
}
