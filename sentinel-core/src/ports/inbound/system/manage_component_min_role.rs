//! Port inbound : gestion des overrides de min_role par composant sensible
//! (`rbac_component_min_role`). Le handler HTTP ne fait que parser/RBAC/valider
//! (registry) puis mapper ; la persistance vit dans le repo outbound.

use async_trait::async_trait;

use crate::domain::entities::system::component_min_role::ComponentMinRoleOverride;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ManageComponentMinRoleUseCase: Send + Sync {
    /// Liste les overrides explicites stockes pour une guild.
    async fn list_overrides(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ComponentMinRoleOverride>, DomainError>;

    /// Lit l'override brut (`min_role` string) d'un composant pour une guild,
    /// s'il existe. `None` = pas d'override (le gate applique alors le default).
    /// Utilise par le middleware `component_gates` pour resoudre le role effectif.
    async fn get_override(
        &self,
        guild_id: &str,
        component_key: &str,
    ) -> Result<Option<String>, DomainError>;

    /// UPSERT idempotent d'un override (guild + component_key -> min_role).
    /// `updated_by` = discord user id de l'auteur.
    async fn upsert(
        &self,
        guild_id: &str,
        component_key: &str,
        min_role: &str,
        updated_by: &str,
    ) -> Result<(), DomainError>;

    /// Supprime l'override (retour au default). Idempotent.
    async fn delete(&self, guild_id: &str, component_key: &str) -> Result<(), DomainError>;
}
