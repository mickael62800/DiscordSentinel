use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{Ticket, TicketMessage};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait TicketRepository: Send + Sync {
    async fn find_all(&self, status: Option<&str>, priority: Option<&str>, search: Option<&str>, author_id: Option<&str>, limit: i64, offset: i64) -> Result<Vec<Ticket>, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Ticket>, DomainError>;
    async fn save(&self, ticket: &Ticket) -> Result<(), DomainError>;
    async fn update_status(&self, id: Uuid, status: &str) -> Result<(), DomainError>;
    async fn update_assignee(&self, id: Uuid, assignee: &str) -> Result<(), DomainError>;
    async fn find_messages(&self, ticket_id: Uuid) -> Result<Vec<TicketMessage>, DomainError>;
    async fn save_message(&self, message: &TicketMessage) -> Result<(), DomainError>;
    async fn update_voice_channel(&self, id: Uuid, voice_channel_id: Option<&str>) -> Result<(), DomainError>;
    async fn update_invited_user(&self, id: Uuid, invited_user_id: Option<&str>) -> Result<(), DomainError>;
}
