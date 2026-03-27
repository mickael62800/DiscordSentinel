use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;

/// Action de modération envoyée au backend.
#[derive(Debug, Serialize)]
pub struct ModerationAction {
    pub guild_id: String,
    pub channel_id: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    /// Gravité pour les warns : "low", "medium", "high"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gravity: Option<String>,
    /// Durée en secondes (None = permanent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ModerationActionResponse {
    pub id: String,
    pub action_type: String,
    pub target_name: String,
    pub reason: String,
}

/// Historique des sanctions d'un utilisateur.
#[derive(Debug, Deserialize)]
pub struct UserHistory {
    pub target_id: String,
    pub target_name: String,
    pub total_warns: u32,
    pub total_mutes: u32,
    pub total_bans: u32,
    pub actions: Vec<ModerationActionResponse>,
}

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    const BOT_NAME: &'static str = "moderation-bot";

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

    /// Enregistre une action de modération dans le backend.
    pub async fn log_action(&self, action: &ModerationAction) -> Result<ModerationActionResponse, String> {
        let req = self
            .client
            .post(format!("{}/api/moderation/actions", self.base_url))
            .json(action);

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?
            .json::<ModerationActionResponse>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    /// Récupère l'historique des sanctions d'un utilisateur.
    pub async fn get_history(&self, guild_id: &str, user_id: &str) -> Result<UserHistory, String> {
        let req = self
            .client
            .get(format!(
                "{}/api/moderation/history/{}/{}",
                self.base_url, guild_id, user_id
            ));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?
            .json::<UserHistory>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
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
