//! Port outbound : persistance des overrides de visibilite des composants UI
//! par role (`rbac_component_visibility`). Tout le SQL (dont la transaction
//! batch) vit dans l'adapter Postgres.

use async_trait::async_trait;

use crate::domain::entities::system::component_visibility::VisibilityEntry;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ComponentVisibilityRepository: Send + Sync {
    /// Liste tous les overrides de visibilite d'une guild.
    async fn list(&self, guild_id: &str) -> Result<Vec<VisibilityEntry>, DomainError>;

    /// UPSERT batch atomique (une seule transaction) de tous les overrides.
    /// `updated_by` = Discord user id de l'auteur.
    async fn upsert_batch(
        &self,
        guild_id: &str,
        entries: &[VisibilityEntry],
        updated_by: &str,
    ) -> Result<(), DomainError>;
}
