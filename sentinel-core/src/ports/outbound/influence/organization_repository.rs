//! Port outbound : persistance des organisations.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::influence::organization::Organization;
use crate::domain::enums::influence::organization_kind::OrganizationKind;
use crate::domain::errors::DomainError;

/// Parametres de creation d'une organisation.
pub struct NewOrganization<'a> {
    pub guild_id: &'a str,
    pub kind: OrganizationKind,
    pub name: &'a str,
    pub motto: &'a str,
    pub founder_id: Uuid,
}

#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    /// Cree une organisation. Renvoie `Conflict` si le nom existe deja.
    async fn create(&self, new: NewOrganization<'_>) -> Result<Organization, DomainError>;

    /// Recupere une organisation par id (active ou dissoute).
    async fn get(&self, id: Uuid) -> Result<Option<Organization>, DomainError>;

    /// Recupere une organisation active par nom (insensible a la casse).
    async fn find_by_name(
        &self,
        guild_id: &str,
        name: &str,
    ) -> Result<Option<Organization>, DomainError>;

    /// Nombre d'organisations actives fondees par ce citoyen.
    async fn count_active_founded_by(&self, founder_id: Uuid) -> Result<i64, DomainError>;

    /// Liste des organisations actives du serveur.
    async fn list_for_guild(&self, guild_id: &str) -> Result<Vec<Organization>, DomainError>;
}
