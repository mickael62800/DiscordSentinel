use std::sync::Arc;

use crate::domain::entities::{Ticket, TicketDetail};
use crate::domain::ports::TicketsRepository;

pub struct TicketsService {
    repo: Arc<dyn TicketsRepository>,
}

impl TicketsService {
    pub fn new(repo: Arc<dyn TicketsRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_tickets(&self) -> Result<Vec<Ticket>, String> {
        self.repo.get_tickets().await
    }

    pub async fn get_ticket_detail(&self, id: String) -> Result<TicketDetail, String> {
        self.repo.get_ticket_detail(id).await
    }

    pub async fn reply_ticket(&self, ticket_id: String, content: String) -> Result<(), String> {
        self.repo.reply_ticket(ticket_id, content).await
    }

    pub async fn close_ticket(&self, id: String) -> Result<(), String> {
        self.repo.close_ticket(id).await
    }

    pub async fn assign_ticket(&self, id: String, assignee: String) -> Result<(), String> {
        self.repo.assign_ticket(id, assignee).await
    }
}
