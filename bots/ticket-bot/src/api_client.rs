use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sentinel_shared::api_client::BaseApiClient;

// ── Tickets ──

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Ticket {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub assigned_to: Option<String>,
    pub server: String,
    pub category: String,
    pub ticket_type: Option<String>,
    pub channel_id: Option<String>,
    pub voice_channel_id: Option<String>,
    pub invited_user_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub messages_count: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TicketMessage {
    pub id: String,
    pub ticket_id: String,
    pub author_name: String,
    pub author_role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TicketDetail {
    pub ticket: Ticket,
    pub messages: Vec<TicketMessage>,
}

#[derive(Debug, Serialize)]
pub struct CreateTicketRequest {
    pub title: String,
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub server: String,
    pub category: String,
    pub ticket_type: String,
    pub channel_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReplyPayload {
    content: String,
    author_name: String,
    author_role: String,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct AssignPayload {
    assignee: String,
}

#[derive(Debug, Serialize)]
struct UpdateTicketChannelPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    voice_channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    invited_user_id: Option<String>,
}

// ── Client ──

pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    pub async fn list_tickets(&self) -> Result<Vec<Ticket>, String> {
        let req = self
            .base
            .client()
            .get(format!("{}/api/tickets", self.base.base_url()));

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<Vec<Ticket>>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    pub async fn create_ticket(&self, request: &CreateTicketRequest) -> Result<Ticket, String> {
        let req = self
            .base
            .client()
            .post(format!("{}/api/tickets", self.base.base_url()))
            .json(request);

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<Ticket>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    #[allow(dead_code)]
    pub async fn get_ticket(&self, id: &str) -> Result<TicketDetail, String> {
        let req = self
            .base
            .client()
            .get(format!("{}/api/tickets/{id}", self.base.base_url()));

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<TicketDetail>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    pub async fn reply_ticket(
        &self,
        ticket_id: &str,
        content: &str,
        author_name: &str,
        author_role: &str,
    ) -> Result<(), String> {
        let req = self
            .base
            .client()
            .post(format!(
                "{}/api/tickets/{ticket_id}/messages",
                self.base.base_url()
            ))
            .json(&ReplyPayload {
                content: content.to_string(),
                author_name: author_name.to_string(),
                author_role: author_role.to_string(),
            });

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    pub async fn update_status(&self, id: &str, status: &str) -> Result<(), String> {
        let req = self
            .base
            .client()
            .patch(format!("{}/api/tickets/{id}/status", self.base.base_url()))
            .json(&serde_json::json!({ "status": status }));

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    pub async fn close_ticket(&self, id: &str) -> Result<(), String> {
        let req = self
            .base
            .client()
            .patch(format!("{}/api/tickets/{id}/close", self.base.base_url()));

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn assign_ticket(&self, id: &str, assignee: &str) -> Result<(), String> {
        let req = self
            .base
            .client()
            .patch(format!("{}/api/tickets/{id}/assign", self.base.base_url()))
            .json(&AssignPayload {
                assignee: assignee.to_string(),
            });

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    pub async fn update_ticket_channel(
        &self,
        id: &str,
        voice_channel_id: Option<String>,
        invited_user_id: Option<String>,
    ) -> Result<(), String> {
        let req = self
            .base
            .client()
            .patch(format!("{}/api/tickets/{id}/channels", self.base.base_url()))
            .json(&UpdateTicketChannelPayload {
                voice_channel_id,
                invited_user_id,
            });

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }
}
