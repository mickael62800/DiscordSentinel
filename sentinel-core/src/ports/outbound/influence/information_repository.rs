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

    /// Fige le resultat (+ lien vers l'info produite si reussie). Garde sur
    /// `status = 'en_cours'` : renvoie `true` si CET appel a reclame l'enquete
    /// (false si une autre execution l'avait deja resolue -> ne rien rejouer).
    async fn resolve(
        &self,
        id: Uuid,
        status: InvestigationStatus,
        info_id: Option<Uuid>,
    ) -> Result<bool, DomainError>;

    /// Attache l'info produite a une enquete deja reclamee (2e temps du succes).
    async fn attach_info(&self, id: Uuid, info_id: Uuid) -> Result<(), DomainError>;
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

    /// Passe une info en public + revelee. Garde sur `revealed = FALSE` :
    /// renvoie `true` si CET appel a effectue la bascule (false si deja revelee
    /// -> l'appelant ne doit PAS rejouer le scandale / la perte de reputation).
    async fn reveal(&self, id: Uuid) -> Result<bool, DomainError>;
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

    /// Dernieres entrees de la memoire du serveur (plus recentes d'abord).
    async fn list_recent(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<crate::domain::entities::influence::archive::ArchiveEntry>, DomainError>;
}
