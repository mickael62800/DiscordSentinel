use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{Ticket, TicketDetail, TicketMessage};
use crate::domain::errors::DomainError;
use crate::ports::inbound::{
    AssignTicketCommand, CreateTicketCommand, ManageTicketsUseCase, ReplyTicketCommand,
};
use crate::ports::outbound::TicketRepository;

pub struct ManageTicketsService {
    ticket_repo: Arc<dyn TicketRepository>,
}

impl ManageTicketsService {
    pub fn new(ticket_repo: Arc<dyn TicketRepository>) -> Self {
        Self { ticket_repo }
    }
}

#[async_trait]
impl ManageTicketsUseCase for ManageTicketsService {
    async fn list_tickets(&self) -> Result<Vec<Ticket>, DomainError> {
        self.ticket_repo.find_all().await
    }

    async fn get_ticket_detail(&self, id: &str) -> Result<TicketDetail, DomainError> {
        let uuid = id
            .parse::<Uuid>()
            .map_err(|_| DomainError::InvalidRule(format!("ID ticket invalide : {id}")))?;

        let ticket = self
            .ticket_repo
            .find_by_id(uuid)
            .await?
            .ok_or(DomainError::Internal(format!("Ticket introuvable : {id}")))?;

        let messages = self.ticket_repo.find_messages(uuid).await?;

        Ok(TicketDetail { ticket, messages })
    }

    async fn create_ticket(&self, cmd: CreateTicketCommand) -> Result<Ticket, DomainError> {
        let now = chrono::Utc::now();
        let ticket = Ticket {
            id: Uuid::new_v4(),
            title: cmd.title,
            status: "open".to_string(),
            priority: cmd.priority,
            author_id: cmd.author_id,
            author_name: cmd.author_name,
            assigned_to: None,
            server: cmd.server,
            category: cmd.category,
            created_at: now,
            updated_at: now,
            messages_count: 0,
        };

        self.ticket_repo.save(&ticket).await?;

        Ok(ticket)
    }

    async fn reply_ticket(&self, cmd: ReplyTicketCommand) -> Result<(), DomainError> {
        let ticket_id = cmd
            .ticket_id
            .parse::<Uuid>()
            .map_err(|_| DomainError::InvalidRule(format!("ID ticket invalide : {}", cmd.ticket_id)))?;

        let message = TicketMessage {
            id: Uuid::new_v4(),
            ticket_id,
            author_name: "staff".to_string(),
            author_role: "moderator".to_string(),
            content: cmd.content,
            created_at: chrono::Utc::now(),
        };

        self.ticket_repo.save_message(&message).await?;

        // Mettre à jour updated_at du ticket
        self.ticket_repo
            .update_status(ticket_id, "pending")
            .await
            .ok();

        Ok(())
    }

    async fn close_ticket(&self, id: &str) -> Result<(), DomainError> {
        let uuid = id
            .parse::<Uuid>()
            .map_err(|_| DomainError::InvalidRule(format!("ID ticket invalide : {id}")))?;

        self.ticket_repo.update_status(uuid, "closed").await
    }

    async fn assign_ticket(&self, cmd: AssignTicketCommand) -> Result<(), DomainError> {
        let uuid = cmd
            .ticket_id
            .parse::<Uuid>()
            .map_err(|_| DomainError::InvalidRule(format!("ID ticket invalide : {}", cmd.ticket_id)))?;

        self.ticket_repo
            .update_assignee(uuid, &cmd.assignee)
            .await
    }
}
