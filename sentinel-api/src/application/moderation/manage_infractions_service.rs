use std::sync::Arc;

use async_trait::async_trait;

use sentinel_core::domain::entities::moderation::infraction::Infraction;
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use crate::ports::outbound::moderation::infraction_repository::InfractionRepository;

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

    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError> {
        self.infraction_repo.delete_older_than_days(guild_id, days).await
    }
}

#[cfg(test)]
#[path = "tests/manage_infractions.rs"]
mod tests;
