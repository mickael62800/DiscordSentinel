use reqwest::{Client, RequestBuilder};
use serde::Serialize;

use crate::config::Config;

#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub guild_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub details: serde_json::Value,
}

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: config.api_base_url.clone(),
            api_key: config.api_key.clone(),
        }
    }

    fn auth(&self, req: RequestBuilder) -> RequestBuilder {
        if self.api_key.is_empty() {
            req
        } else {
            req.bearer_auth(&self.api_key)
        }
    }

    pub async fn send_audit_event(&self, event: &AuditEvent) -> Result<(), String> {
        let req = self
            .client
            .post(format!("{}/api/audit-logs", self.base_url))
            .json(event);

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    pub async fn heartbeat(&self, bot_name: &str) {
        #[derive(Serialize)]
        struct Payload {
            bot_name: String,
        }

        let req = self
            .client
            .post(format!("{}/api/bots/heartbeat", self.base_url))
            .json(&Payload {
                bot_name: bot_name.to_string(),
            });

        let _ = self.auth(req).send().await;
    }

    pub async fn register_guild(
        &self,
        guild_id: &str,
        name: &str,
        member_count: i32,
    ) -> Result<(), String> {
        #[derive(Serialize)]
        struct Payload {
            guild_id: String,
            name: String,
            member_count: Option<i32>,
        }

        let req = self
            .client
            .post(format!("{}/api/guilds/register", self.base_url))
            .json(&Payload {
                guild_id: guild_id.to_string(),
                name: name.to_string(),
                member_count: Some(member_count),
            });

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Guild register failed: {e}"))?;

        Ok(())
    }
}
