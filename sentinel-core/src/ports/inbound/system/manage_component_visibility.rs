//! Port inbound : gestion des overrides de visibilite des composants UI par
//! role (`rbac_component_visibility`). Le handler HTTP ne fait que
//! parser/RBAC/valider/mapper ; le SQL (dont la transaction batch) vit dans
//! `ComponentVisibilityRepository`.

use async_trait::async_trait;

use crate::domain::entities::system::component_visibility::VisibilityEntry;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ManageComponentVisibilityUseCase: Send + Sync {
    /// Liste tous les overrides de visibilite d'une guild.
    async fn list(&self, guild_id: &str) -> Result<Vec<VisibilityEntry>, DomainError>;

    /// UPSERT batch de tous les overrides envoyes (transaction atomique).
    /// `updated_by` = Discord user id de l'auteur. Renvoie le nombre d'entrees.
    async fn upsert_batch(
        &self,
        guild_id: &str,
        entries: Vec<VisibilityEntry>,
        updated_by: &str,
    ) -> Result<usize, DomainError>;
}
