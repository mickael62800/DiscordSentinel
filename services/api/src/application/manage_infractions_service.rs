use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::Infraction;
use crate::domain::errors::DomainError;
use crate::ports::inbound::{InfractionFilters, ManageInfractionsUseCase};
use crate::ports::outbound::InfractionRepository;

pub struct ManageInfractionsService {
    infraction_repo: Arc<dyn InfractionRepository>,
}

impl ManageInfractionsService {
    pub fn new(infraction_repo: Arc<dyn InfractionRepository>) -> Self {
        Self { infraction_repo }
    }
}

#[async_trait]
impl ManageInfractionsUseCase for ManageInfractionsService {
    async fn list_infractions(
        &self,
        guild_id: &str,
        filters: InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError> {
        self.infraction_repo.find_by_guild(guild_id, &filters).await
    }

    async fn list_all_infractions(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Infraction>, DomainError> {
        self.infraction_repo.find_all(limit, offset).await
    }

    async fn count_today(&self) -> Result<u64, DomainError> {
        self.infraction_repo.count_today().await
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Infraction>, DomainError> {
        self.infraction_repo.find_by_id(id).await
    }

    async fn delete_infraction(&self, id: &str) -> Result<bool, DomainError> {
        self.infraction_repo.delete_by_id(id).await
    }
}
