use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::{Ticket, TicketMessage};
use crate::domain::errors::DomainError;
use crate::ports::outbound::TicketRepository;

pub struct PgTicketRepository {
    pool: PgPool,
}

impl PgTicketRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct TicketRow {
    id: Uuid,
    title: String,
    status: String,
    priority: String,
    author_id: String,
    author_name: String,
    assigned_to: Option<String>,
    server: String,
    category: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    messages_count: Option<i64>,
}

impl From<TicketRow> for Ticket {
    fn from(row: TicketRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            status: row.status,
            priority: row.priority,
            author_id: row.author_id,
            author_name: row.author_name,
            assigned_to: row.assigned_to,
            server: row.server,
            category: row.category,
            created_at: row.created_at,
            updated_at: row.updated_at,
            messages_count: row.messages_count.unwrap_or(0) as u32,
        }
    }
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: Uuid,
    ticket_id: Uuid,
    author_name: String,
    author_role: String,
    content: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<MessageRow> for TicketMessage {
    fn from(row: MessageRow) -> Self {
        Self {
            id: row.id,
            ticket_id: row.ticket_id,
            author_name: row.author_name,
            author_role: row.author_role,
            content: row.content,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl TicketRepository for PgTicketRepository {
    async fn find_all(&self) -> Result<Vec<Ticket>, DomainError> {
        let rows = sqlx::query_as::<_, TicketRow>(
            r#"
            SELECT t.*, (SELECT COUNT(*) FROM ticket_messages WHERE ticket_id = t.id) AS messages_count
            FROM tickets t
            ORDER BY t.updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(Ticket::from).collect())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Ticket>, DomainError> {
        let row = sqlx::query_as::<_, TicketRow>(
            r#"
            SELECT t.*, (SELECT COUNT(*) FROM ticket_messages WHERE ticket_id = t.id) AS messages_count
            FROM tickets t
            WHERE t.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(Ticket::from))
    }

    async fn save(&self, ticket: &Ticket) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO tickets (id, title, status, priority, author_id, author_name, assigned_to, server, category, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(ticket.id)
        .bind(&ticket.title)
        .bind(&ticket.status)
        .bind(&ticket.priority)
        .bind(&ticket.author_id)
        .bind(&ticket.author_name)
        .bind(&ticket.assigned_to)
        .bind(&ticket.server)
        .bind(&ticket.category)
        .bind(ticket.created_at)
        .bind(ticket.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_status(&self, id: Uuid, status: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE tickets SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_assignee(&self, id: Uuid, assignee: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE tickets SET assigned_to = $1, updated_at = NOW() WHERE id = $2")
            .bind(assignee)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn find_messages(&self, ticket_id: Uuid) -> Result<Vec<TicketMessage>, DomainError> {
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT * FROM ticket_messages WHERE ticket_id = $1 ORDER BY created_at ASC",
        )
        .bind(ticket_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(TicketMessage::from).collect())
    }

    async fn save_message(&self, message: &TicketMessage) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ticket_messages (id, ticket_id, author_name, author_role, content, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(message.id)
        .bind(message.ticket_id)
        .bind(&message.author_name)
        .bind(&message.author_role)
        .bind(&message.content)
        .bind(message.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }
}
