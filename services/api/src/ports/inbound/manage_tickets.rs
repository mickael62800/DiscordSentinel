use async_trait::async_trait;

use crate::domain::entities::{Ticket, TicketDetail};
use crate::domain::errors::DomainError;

pub struct CreateTicketCommand {
    pub title: String,
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub server: String,
    pub category: String,
}

pub struct ReplyTicketCommand {
    pub ticket_id: String,
    pub content: String,
}

pub struct AssignTicketCommand {
    pub ticket_id: String,
    pub assignee: String,
}

#[async_trait]
pub trait ManageTicketsUseCase: Send + Sync {
    async fn list_tickets(&self) -> Result<Vec<Ticket>, DomainError>;
    async fn get_ticket_detail(&self, id: &str) -> Result<TicketDetail, DomainError>;
    async fn create_ticket(&self, command: CreateTicketCommand) -> Result<Ticket, DomainError>;
    async fn reply_ticket(&self, command: ReplyTicketCommand) -> Result<(), DomainError>;
    async fn close_ticket(&self, id: &str) -> Result<(), DomainError>;
    async fn assign_ticket(&self, command: AssignTicketCommand) -> Result<(), DomainError>;
}
