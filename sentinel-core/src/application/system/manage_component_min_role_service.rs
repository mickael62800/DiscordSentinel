//! Use case des overrides de min_role par composant : delegue la persistance au
//! repo. La validation metier (registry des cles, floor) reste cote API
//! (`middleware/component_gates`) ; ici on ne fait que router vers le repo.

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::system::component_min_role::ComponentMinRoleOverride;
use crate::domain::errors::DomainError;
use crate::ports::inbound::system::manage_component_min_role::ManageComponentMinRoleUseCase;
use crate::ports::outbound::system::component_min_role_repository::ComponentMinRoleRepository;

pub struct ManageComponentMinRoleService {
    repo: Arc<dyn ComponentMinRoleRepository>,
}

impl ManageComponentMinRoleService {
    pub fn new(repo: Arc<dyn ComponentMinRoleRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageComponentMinRoleUseCase for ManageComponentMinRoleService {
    async fn list_overrides(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ComponentMinRoleOverride>, DomainError> {
        self.repo.list_for_guild(guild_id).await
    }

    async fn upsert(
        &self,
        guild_id: &str,
        component_key: &str,
        min_role: &str,
        updated_by: &str,
    ) -> Result<(), DomainError> {
        self.repo
            .upsert(guild_id, component_key, min_role, updated_by)
            .await
    }

    async fn delete(&self, guild_id: &str, component_key: &str) -> Result<(), DomainError> {
        self.repo.delete(guild_id, component_key).await
    }
}
