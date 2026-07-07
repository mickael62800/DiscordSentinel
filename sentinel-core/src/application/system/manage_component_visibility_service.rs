//! Use case Component Visibility : delegue la persistance au repo (le SQL et la
//! transaction batch vivent dans `ComponentVisibilityRepository`). Le handler
//! HTTP ne fait que parser/RBAC/valider/mapper.

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::system::component_visibility::VisibilityEntry;
use crate::domain::errors::DomainError;
use crate::ports::inbound::system::manage_component_visibility::ManageComponentVisibilityUseCase;
use crate::ports::outbound::system::component_visibility_repository::ComponentVisibilityRepository;

pub struct ManageComponentVisibilityService {
    repo: Arc<dyn ComponentVisibilityRepository>,
}

impl ManageComponentVisibilityService {
    pub fn new(repo: Arc<dyn ComponentVisibilityRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageComponentVisibilityUseCase for ManageComponentVisibilityService {
    async fn list(&self, guild_id: &str) -> Result<Vec<VisibilityEntry>, DomainError> {
        self.repo.list(guild_id).await
    }

    async fn upsert_batch(
        &self,
        guild_id: &str,
        entries: Vec<VisibilityEntry>,
        updated_by: &str,
    ) -> Result<usize, DomainError> {
        self.repo.upsert_batch(guild_id, &entries, updated_by).await?;
        Ok(entries.len())
    }
}
