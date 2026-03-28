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
    const BOT_NAME: &'static str = "ticket-bot";

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

    pub fn send_log(&self, level: &str, server: &str, message: &str) {
        self.send_log_with_category(level, server, message, "discord");
    }

    pub fn send_bot_log(&self, level: &str, message: &str) {
        self.send_log_with_category(level, "", message, "bot");
    }

    fn send_log_with_category(&self, level: &str, server: &str, message: &str, category: &str) {
        #[derive(serde::Serialize)]
        struct LogPayload {
            level: String,
            bot: String,
            server: String,
            message: String,
            category: String,
        }

        let req = self.auth(
            self.client
                .post(format!("{}/api/logs", self.base_url))
                .json(&LogPayload {
                    level: level.to_string(),
                    bot: Self::BOT_NAME.to_string(),
                    server: server.to_string(),
                    message: message.to_string(),
                    category: category.to_string(),
                }),
        );

        tokio::spawn(async move {
            let _ = req.send().await;
        });
    }

    pub async fn heartbeat(&self, name: &str) -> Result<(), String> {
        #[derive(serde::Serialize)]
        struct Payload { name: String }

        let mut req = self.client
            .post(format!("{}/api/bots/heartbeat", self.base_url))
            .json(&Payload { name: name.to_string() });

        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        req.send().await.map_err(|e| format!("Heartbeat failed: {e}"))?;
        Ok(())
    }

    pub async fn get_guild_config(&self, guild_id: &str) -> Result<std::collections::HashMap<String, String>, String> {
        let url = format!("{}/api/bots/config/{}/{}", self.base_url, guild_id, Self::BOT_NAME);
        let mut req = self.client.get(&url);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        #[derive(serde::Deserialize)]
        struct ConfigEntry {
            config_key: String,
            config_value: String,
        }

        let resp = req.send().await.map_err(|e| format!("Config fetch failed: {e}"))?;
        let entries: Vec<ConfigEntry> = resp.json().await.map_err(|e| format!("Config parse failed: {e}"))?;
        Ok(entries.into_iter().map(|e| (e.config_key, e.config_value)).collect())
    }

    /// Helper pour lire une valeur de config avec fallback
    pub fn config_or(config: &std::collections::HashMap<String, String>, key: &str, default: &str) -> String {
        config.get(key).cloned().unwrap_or_else(|| default.to_string())
    }

    pub fn config_u64(config: &std::collections::HashMap<String, String>, key: &str, default: u64) -> u64 {
        config.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
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

    pub async fn reply_ticket(
        &self,
        ticket_id: &str,
        content: &str,
        author_name: &str,
        author_role: &str,
    ) -> Result<(), String> {
        let req = self
            .client
            .post(format!("{}/api/tickets/{ticket_id}/messages", self.base_url))
            .json(&ReplyPayload {
                content: content.to_string(),
                author_name: author_name.to_string(),
                author_role: author_role.to_string(),
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

    pub async fn register_guild(&self, guild_id: &str, name: &str, member_count: i32) -> Result<(), String> {
        #[derive(serde::Serialize)]
        struct Payload {
            guild_id: String,
            name: String,
            member_count: Option<i32>,
        }

        let mut req = self.client
            .post(format!("{}/api/guilds/register", self.base_url))
            .json(&Payload {
                guild_id: guild_id.to_string(),
                name: name.to_string(),
                member_count: Some(member_count),
            });

        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        req.send().await.map_err(|e| format!("Guild register failed: {e}"))?;
        Ok(())
    }
}
