use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;

/// Payload envoye au backend pour analyse d'image.
#[derive(Debug, Serialize)]
pub struct AnalyzeImageRequest {
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub message_id: String,
    /// Image encodee en base64
    pub image_data: String,
    /// Type MIME de l'image (image/png, image/jpeg, etc.)
    pub content_type: String,
    /// Nom du fichier original
    pub filename: String,
}

/// Reponse du backend apres analyse d'image.
#[derive(Debug, Deserialize)]
pub struct AnalyzeImageResponse {
    pub action: Action,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub classifications: Vec<Classification>,
}

#[derive(Debug, Deserialize)]
pub struct Classification {
    pub label: String,
    pub confidence: f32,
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
    max_image_size: u64,
}

impl ApiClient {
    const BOT_NAME: &'static str = "image-bot";

    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            base_url: config.api_base_url.clone(),
            api_key: config.api_key.clone(),
            max_image_size: config.max_image_size,
        }
    }

    pub fn max_image_size(&self) -> u64 {
        self.max_image_size
    }

    /// Envoie un heartbeat au backend.
    pub async fn heartbeat(&self, name: &str) -> Result<(), String> {
        #[derive(serde::Serialize)]
        struct Payload {
            name: String,
        }

        let mut req = self
            .client
            .post(format!("{}/api/bots/heartbeat", self.base_url))
            .json(&Payload {
                name: name.to_string(),
            });

        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        req.send()
            .await
            .map_err(|e| format!("Heartbeat failed: {e}"))?;
        Ok(())
    }

    /// Envoie une image au backend pour analyse (NSFW / produits illicites).
    pub async fn analyze_image(
        &self,
        request: &AnalyzeImageRequest,
    ) -> Result<AnalyzeImageResponse, reqwest::Error> {
        let mut req = self
            .client
            .post(format!("{}/analyze/image", self.base_url))
            .json(request);

        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        req.send().await?.json().await
    }

    /// Telecharge une image depuis une URL (attachment Discord).
    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>, reqwest::Error> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Recupere la config per-guild depuis l'API.
    pub async fn get_guild_config(
        &self,
        guild_id: &str,
    ) -> Result<std::collections::HashMap<String, String>, String> {
        let url = format!(
            "{}/api/bots/config/{}/{}",
            self.base_url,
            guild_id,
            Self::BOT_NAME
        );
        let mut req = self.client.get(&url);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        #[derive(serde::Deserialize)]
        struct ConfigEntry {
            config_key: String,
            config_value: String,
        }

        let resp = req
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

    /// Enregistre une guild aupres de l'API.
    pub async fn register_guild(
        &self,
        guild_id: &str,
        name: &str,
        member_count: i32,
    ) -> Result<(), String> {
        #[derive(serde::Serialize)]
        struct Payload {
            guild_id: String,
            name: String,
            member_count: Option<i32>,
        }

        let mut req = self
            .client
            .post(format!("{}/api/guilds/register", self.base_url))
            .json(&Payload {
                guild_id: guild_id.to_string(),
                name: name.to_string(),
                member_count: Some(member_count),
            });

        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        req.send()
            .await
            .map_err(|e| format!("Guild register failed: {e}"))?;
        Ok(())
    }
}
