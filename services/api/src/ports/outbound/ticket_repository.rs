use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{Ticket, TicketMessage};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait TicketRepository: Send + Sync {
    async fn find_all(&self) -> Result<Vec<Ticket>, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Ticket>, DomainError>;
    async fn save(&self, ticket: &Ticket) -> Result<(), DomainError>;
    async fn update_status(&self, id: Uuid, status: &str) -> Result<(), DomainError>;
    async fn update_assignee(&self, id: Uuid, assignee: &str) -> Result<(), DomainError>;
    async fn find_messages(&self, ticket_id: Uuid) -> Result<Vec<TicketMessage>, DomainError>;
    async fn save_message(&self, message: &TicketMessage) -> Result<(), DomainError>;
}
