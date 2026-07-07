//! Port outbound : persistance des overrides de min_role par composant
//! (`rbac_component_min_role`). Tout le SQL vit dans l'adapter Postgres.

use async_trait::async_trait;

use crate::domain::entities::system::component_min_role::ComponentMinRoleOverride;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ComponentMinRoleRepository: Send + Sync {
    /// Liste les overrides explicites stockes pour une guild.
    async fn list_for_guild(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ComponentMinRoleOverride>, DomainError>;

    /// UPSERT idempotent d'un override (guild + component_key -> min_role).
    async fn upsert(
        &self,
        guild_id: &str,
        component_key: &str,
        min_role: &str,
        updated_by: &str,
    ) -> Result<(), DomainError>;

    /// Supprime l'override d'une guild (idempotent).
    async fn delete(&self, guild_id: &str, component_key: &str) -> Result<(), DomainError>;
}
