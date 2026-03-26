use serde::{Deserialize, Serialize};

use crate::domain::entities::{Ticket, TicketDetail, TicketMessage};
use crate::ports::inbound::CreateTicketCommand;

#[derive(Debug, Deserialize)]
pub struct CreateTicketDto {
    pub title: String,
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub server: String,
    pub category: String,
}

#[derive(Debug, Deserialize)]
pub struct ReplyDto {
    pub content: String,
    #[serde(default = "default_author_name")]
    pub author_name: String,
    #[serde(default = "default_author_role")]
    pub author_role: String,
}

fn default_author_name() -> String {
    "staff".to_string()
}

fn default_author_role() -> String {
    "moderator".to_string()
}

#[derive(Debug, Deserialize)]
pub struct AssignDto {
    pub assignee: String,
}

#[derive(Debug, Serialize)]
pub struct TicketResponseDto {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub assigned_to: Option<String>,
    pub server: String,
    pub category: String,
    pub created_at: String,
    pub updated_at: String,
    pub messages_count: u32,
}

#[derive(Debug, Serialize)]
pub struct TicketMessageDto {
    pub id: String,
    pub ticket_id: String,
    pub author_name: String,
    pub author_role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct TicketDetailDto {
    pub ticket: TicketResponseDto,
    pub messages: Vec<TicketMessageDto>,
}

impl From<CreateTicketDto> for CreateTicketCommand {
    fn from(dto: CreateTicketDto) -> Self {
        Self {
            title: dto.title,
            priority: dto.priority,
            author_id: dto.author_id,
            author_name: dto.author_name,
            server: dto.server,
            category: dto.category,
        }
    }
}

impl From<Ticket> for TicketResponseDto {
    fn from(t: Ticket) -> Self {
        Self {
            id: t.id.to_string(),
            title: t.title,
            status: t.status,
            priority: t.priority,
            author_id: t.author_id,
            author_name: t.author_name,
            assigned_to: t.assigned_to,
            server: t.server,
            category: t.category,
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
            messages_count: t.messages_count,
        }
    }
}

impl From<TicketMessage> for TicketMessageDto {
    fn from(m: TicketMessage) -> Self {
        Self {
            id: m.id.to_string(),
            ticket_id: m.ticket_id.to_string(),
            author_name: m.author_name,
            author_role: m.author_role,
            content: m.content,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

impl From<TicketDetail> for TicketDetailDto {
    fn from(d: TicketDetail) -> Self {
        Self {
            ticket: TicketResponseDto::from(d.ticket),
            messages: d.messages.into_iter().map(TicketMessageDto::from).collect(),
        }
    }
}
