use std::collections::HashMap;
use std::time::Duration;

use reqwest::{Client, RequestBuilder};
use serde::Serialize;

use crate::config::BotConfig;

/// Client HTTP de base partage entre tous les bots.
/// Fournit : heartbeat, register_guild, send_log, get_guild_config, config helpers.
pub struct BaseApiClient {
    client: Client,
    base_url: String,
    api_key: String,
    bot_name: String,
}

impl BaseApiClient {
    pub fn new<C: BotConfig>(config: &C, bot_name: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: config.api_base_url().to_string(),
            api_key: config.api_key().to_string(),
            bot_name: bot_name.to_string(),
        }
    }

    /// Retourne le client HTTP pour les requetes specifiques au bot.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Retourne l'URL de base de l'API.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Retourne le nom du bot.
    pub fn bot_name(&self) -> &str {
        &self.bot_name
    }

    /// Ajoute l'authentification Bearer si une cle API est configuree.
    pub fn auth(&self, req: RequestBuilder) -> RequestBuilder {
        if self.api_key.is_empty() {
            req
        } else {
            req.bearer_auth(&self.api_key)
        }
    }

    // ── Heartbeat ──

    pub async fn heartbeat(&self) -> Result<(), String> {
        #[derive(Serialize)]
        struct Payload {
            name: String,
        }

        let req = self
            .client
            .post(format!("{}/api/bots/heartbeat", self.base_url))
            .json(&Payload {
                name: self.bot_name.clone(),
            });

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Heartbeat failed: {e}"))?;

        Ok(())
    }

    // ── Guild Registration ──

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

    // ── Logging ──

    pub fn send_log(&self, level: &str, server: &str, message: &str) {
        self.send_log_with_category(level, server, message, "discord");
    }

    pub fn send_bot_log(&self, level: &str, message: &str) {
        self.send_log_with_category(level, "", message, "bot");
    }

    fn send_log_with_category(
        &self,
        level: &str,
        server: &str,
        message: &str,
        category: &str,
    ) {
        #[derive(Serialize)]
        struct LogPayload {
            level: String,
            bot: String,
            server: String,
            message: String,
            category: String,
        }

        let req = self
            .client
            .post(format!("{}/api/logs", self.base_url))
            .json(&LogPayload {
                level: level.to_string(),
                bot: self.bot_name.clone(),
                server: server.to_string(),
                message: message.to_string(),
                category: category.to_string(),
            });

        let req = self.auth(req);
        tokio::spawn(async move {
            if let Err(e) = req.send().await {
                tracing::warn!("Log send failed: {e}");
            }
        });
    }

    // ── Guild Config ──

    pub async fn get_guild_config(
        &self,
        guild_id: &str,
    ) -> Result<HashMap<String, String>, String> {
        let url = format!(
            "{}/api/bots/config/{}/{}",
            self.base_url, guild_id, self.bot_name
        );
        let req = self.client.get(&url);

        #[derive(serde::Deserialize)]
        struct ConfigEntry {
            config_key: String,
            config_value: String,
        }

        let resp = self
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Config fetch failed: {e}"))?;

        let entries: Vec<ConfigEntry> = resp
            .json()
            .await
            .map_err(|e| format!("Config parse failed: {e}"))?;

        Ok(entries
            .into_iter()
            .map(|e| (e.config_key, e.config_value))
            .collect())
    }

    // ── Config Helpers ──

    pub fn config_or(
        config: &HashMap<String, String>,
        key: &str,
        default: &str,
    ) -> String {
        config
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    pub fn config_u64(
        config: &HashMap<String, String>,
        key: &str,
        default: u64,
    ) -> u64 {
        config
            .get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    pub fn config_bool(
        config: &HashMap<String, String>,
        key: &str,
        default: bool,
    ) -> bool {
        config
            .get(key)
            .map(|v| v == "true" || v == "1")
            .unwrap_or(default)
    }
}
