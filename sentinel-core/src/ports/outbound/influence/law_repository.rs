//! Port outbound : persistance des lois.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::influence::law::{Law, LawStatus};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait LawRepository: Send + Sync {
    /// Cree une loi en statut « vote » avec une echeance.
    async fn create(
        &self,
        guild_id: &str,
        title: &str,
        body: &str,
        author_id: Uuid,
        closes_at: DateTime<Utc>,
    ) -> Result<Law, DomainError>;

    async fn get(&self, id: Uuid) -> Result<Option<Law>, DomainError>;

    /// Memorise le message Discord (pour edition a la cloture).
    async fn set_message(
        &self,
        id: Uuid,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), DomainError>;

    /// Fige le resultat d'une loi.
    /// Cloture une loi. Garde sur `status = 'vote'` : renvoie `true` si CET
    /// appel a bien cloture (false si une autre execution l'avait deja fait ->
    /// ne pas archiver / rejouer l'effet en double).
    async fn close(&self, id: Uuid, status: LawStatus) -> Result<bool, DomainError>;

    /// Lois en vote dont l'echeance est passee (scan worker).
    async fn list_due(&self, now: DateTime<Utc>) -> Result<Vec<Law>, DomainError>;
}
