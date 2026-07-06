//! Port outbound : persistance des adhesions aux organisations.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::influence::org_membership::{OrgMember, OrgMemberView, OrgRole};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait MembershipRepository: Send + Sync {
    /// Ajoute un membre. Renvoie `Conflict` s'il est deja membre.
    async fn add(&self, org_id: Uuid, citizen_id: Uuid, role: OrgRole)
        -> Result<(), DomainError>;

    /// Recupere l'adhesion d'un citoyen a une org (s'il est membre).
    async fn get(
        &self,
        org_id: Uuid,
        citizen_id: Uuid,
    ) -> Result<Option<OrgMember>, DomainError>;

    /// Liste des membres (avec pseudo) pour l'affichage, tries par rang.
    async fn list_views(&self, org_id: Uuid) -> Result<Vec<OrgMemberView>, DomainError>;

    /// Nombre de membres d'une organisation.
    async fn count(&self, org_id: Uuid) -> Result<i64, DomainError>;

    /// (user_id Discord, username) de chaque membre — pour crediter leurs wallets
    /// (dividendes).
    async fn list_member_user_ids(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<(String, String)>, DomainError>;
}
