use async_trait::async_trait;

use crate::domain::entities::moderation::user_note::UserNote;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait NotesRepository: Send + Sync {
    async fn save(&self, note: &UserNote) -> Result<(), DomainError>;
    async fn find_by_user(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<UserNote>, DomainError>;
    async fn delete(&self, note_id: &str) -> Result<(), DomainError>;
}
