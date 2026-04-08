use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::BlackjackGame;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait BlackjackRepository: Send + Sync {
    async fn create(&self, game: &BlackjackGame) -> Result<(), DomainError>;
    async fn get_active(&self, guild_id: &str, user_id: &str) -> Result<Option<BlackjackGame>, DomainError>;
    async fn update(&self, game: &BlackjackGame) -> Result<(), DomainError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<BlackjackGame>, DomainError>;
}
