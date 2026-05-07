use async_trait::async_trait;
use uuid::Uuid;

use sentinel_core::domain::entities::game::template::GameTemplate;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait GameTemplateRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<GameTemplate>, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<GameTemplate>, DomainError>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<GameTemplate>, DomainError>;
}
