use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;

// ── Tickets ──

#[derive(Debug, Deserialize)]
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
    pub created_at: String,
    pub updated_at: String,
    pub messages_count: u32,
}

#[derive(Debug, Deserialize)]
pub struct TicketMessage {
    pub id: String,
    pub ticket_id: String,
    pub author_name: String,
    pub author_role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
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
}

#[derive(Debug, Serialize)]
struct ReplyPayload {
    content: String,
}

#[derive(Debug, Serialize)]
struct AssignPayload {
    assignee: String,
}

// ── Client ──

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            base_url: config.api_base_url.clone(),
            api_key: config.api_key.clone(),
        }
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.api_key.is_empty() {
            req
        } else {
            req.bearer_auth(&self.api_key)
        }
    }

    pub async fn create_ticket(&self, request: &CreateTicketRequest) -> Result<Ticket, String> {
        let req = self
            .client
            .post(format!("{}/api/tickets", self.base_url))
            .json(request);

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?
            .json::<Ticket>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    pub async fn get_ticket(&self, id: &str) -> Result<TicketDetail, String> {
        let req = self
            .client
            .get(format!("{}/api/tickets/{id}", self.base_url));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?
            .json::<TicketDetail>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    pub async fn reply_ticket(&self, ticket_id: &str, content: &str) -> Result<(), String> {
        let req = self
            .client
            .post(format!("{}/api/tickets/{ticket_id}/messages", self.base_url))
            .json(&ReplyPayload {
                content: content.to_string(),
            });

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?;

        Ok(())
    }

    pub async fn close_ticket(&self, id: &str) -> Result<(), String> {
        let req = self
            .client
            .patch(format!("{}/api/tickets/{id}/close", self.base_url));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?;

        Ok(())
    }

    pub async fn assign_ticket(&self, id: &str, assignee: &str) -> Result<(), String> {
        let req = self
            .client
            .patch(format!("{}/api/tickets/{id}/assign", self.base_url))
            .json(&AssignPayload {
                assignee: assignee.to_string(),
            });

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?;

        Ok(())
    }
}
