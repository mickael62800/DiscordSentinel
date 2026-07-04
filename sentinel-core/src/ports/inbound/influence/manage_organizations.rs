//! Use case : gestion des organisations (creation, info, adhesion, membres).

use async_trait::async_trait;

use crate::domain::entities::influence::org_membership::OrgMemberView;
use crate::domain::entities::influence::organization::Organization;
use crate::domain::enums::influence::organization_kind::OrganizationKind;
use crate::domain::errors::DomainError;

/// Vue d'ensemble d'une organisation (`/org info`).
#[derive(Debug, Clone)]
pub struct OrgInfo {
    pub org: Organization,
    pub member_count: i64,
}

#[async_trait]
pub trait ManageOrganizationsUseCase: Send + Sync {
    /// Fonde une organisation : verifie le quota et le cout (debite l'Argent du
    /// fondateur), cree l'org et l'adhesion Fondateur.
    async fn create(
        &self,
        guild_id: &str,
        founder_user_id: &str,
        founder_username: &str,
        kind: OrganizationKind,
        name: &str,
        motto: &str,
    ) -> Result<Organization, DomainError>;

    /// Informations sur une organisation par nom.
    async fn info(&self, guild_id: &str, name: &str) -> Result<OrgInfo, DomainError>;

    /// Rejoint une organisation comme Recrue.
    async fn join(
        &self,
        guild_id: &str,
        name: &str,
        user_id: &str,
        username: &str,
    ) -> Result<Organization, DomainError>;

    /// Liste des membres d'une organisation (pour affichage).
    async fn members(
        &self,
        guild_id: &str,
        name: &str,
    ) -> Result<Vec<OrgMemberView>, DomainError>;
}
