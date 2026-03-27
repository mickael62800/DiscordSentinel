use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::detectors::DetectionFlags;

/// Payload envoyé au backend pour analyse.
#[derive(Debug, Serialize)]
pub struct AnalyzeRequest {
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub content: String,
    pub flags: DetectionFlags,
    pub metadata: MessageMetadata,
}

#[derive(Debug, Serialize)]
pub struct MessageMetadata {
    pub message_id: String,
    pub timestamp: String,
}

/// Réponse du backend.
#[derive(Debug, Deserialize)]
pub struct AnalyzeResponse {
    pub action: Action,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub duration: Option<u64>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    None,
    Warn,
    Delete,
    Mute,
    Ban,
}

/// Client HTTP pour communiquer avec le backend.
pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    const BOT_NAME: &'static str = "automod-bot";

    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            base_url: config.api_base_url.clone(),
            api_key: config.api_key.clone(),
        }
    }

    /// Envoie un message au backend pour analyse et retourne l'action à effectuer.
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

    /// Envoie un message au backend pour analyse et retourne l'action à effectuer.
    pub async fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzeResponse, reqwest::Error> {
        let mut req = self
            .client
            .post(format!("{}/analyze", self.base_url))
            .json(request);

        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        req.send().await?.json().await
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
