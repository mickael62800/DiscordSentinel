//! Port outbound : relations inter-organisations (Phase 5).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::influence::archive::{OrgRelation, RelationKind};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait RelationRepository: Send + Sync {
    /// Definit (ou remplace) la relation dirigee de `org_id` vers `other_org_id`.
    async fn set(
        &self,
        guild_id: &str,
        org_id: Uuid,
        other_org_id: Uuid,
        relation: RelationKind,
    ) -> Result<(), DomainError>;

    /// Relations declarees par une organisation.
    async fn list_for_org(&self, org_id: Uuid) -> Result<Vec<OrgRelation>, DomainError>;
}
