//! Ports outbound : enquetes, informations, archives.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::influence::information::{
    Information, Investigation, InvestigationStatus,
};
use crate::domain::errors::DomainError;

/// Parametres d'ouverture d'une enquete.
pub struct NewInvestigation<'a> {
    pub guild_id: &'a str,
    pub initiator_id: Uuid,
    pub initiator_user_id: &'a str,
    pub target_user_id: &'a str,
    pub target_username: &'a str,
    pub subject: &'a str,
    pub resolves_at: DateTime<Utc>,
}

#[async_trait]
pub trait InvestigationRepository: Send + Sync {
    async fn create(&self, new: NewInvestigation<'_>) -> Result<Investigation, DomainError>;

    /// Enquetes arrivees a echeance (scan worker).
    async fn list_due(&self, now: DateTime<Utc>) -> Result<Vec<Investigation>, DomainError>;

    /// Fige le resultat (+ lien vers l'info produite si reussie).
    async fn resolve(
        &self,
        id: Uuid,
        status: InvestigationStatus,
        info_id: Option<Uuid>,
    ) -> Result<(), DomainError>;
}

/// Parametres de creation d'une information.
pub struct NewInformation<'a> {
    pub guild_id: &'a str,
    pub owner_id: Uuid,
    pub target_user_id: &'a str,
    pub target_username: &'a str,
    pub content: &'a str,
}

#[async_trait]
pub trait InformationRepository: Send + Sync {
    /// Cree une information secrete.
    async fn create_secret(&self, new: NewInformation<'_>) -> Result<Uuid, DomainError>;

    async fn get(&self, id: Uuid) -> Result<Option<Information>, DomainError>;

    /// Intel secret non revele detenu par un citoyen.
    async fn list_secret_for_owner(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<Information>, DomainError>;

    /// Passe une info en public + revelee.
    async fn reveal(&self, id: Uuid) -> Result<(), DomainError>;
}

#[async_trait]
pub trait ArchiveRepository: Send + Sync {
    /// Ajoute une entree immuable a la memoire du serveur.
    async fn append(
        &self,
        guild_id: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), DomainError>;
}
