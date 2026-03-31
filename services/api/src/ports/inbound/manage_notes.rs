use async_trait::async_trait;

use crate::domain::entities::UserNote;
use crate::domain::errors::DomainError;

pub struct AddNoteCommand {
    pub guild_id: String,
    pub user_id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub category: String,
}

#[async_trait]
pub trait ManageNotesUseCase: Send + Sync {
    async fn add_note(&self, cmd: AddNoteCommand) -> Result<UserNote, DomainError>;
    async fn get_notes(&self, guild_id: &str, user_id: &str) -> Result<Vec<UserNote>, DomainError>;
    async fn delete_note(&self, note_id: &str) -> Result<(), DomainError>;
}
