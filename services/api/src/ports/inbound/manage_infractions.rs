use async_trait::async_trait;

use crate::domain::entities::Infraction;
use crate::domain::errors::DomainError;

pub struct InfractionFilters {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

#[async_trait]
#[allow(dead_code)]
pub trait ManageInfractionsUseCase: Send + Sync {
    async fn list_infractions(
        &self,
        guild_id: &str,
        filters: InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError>;

    async fn list_all_infractions(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Infraction>, DomainError>;

    async fn count_today(&self) -> Result<u64, DomainError>;
}
