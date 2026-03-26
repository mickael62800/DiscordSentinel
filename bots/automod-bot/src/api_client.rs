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
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            base_url: config.api_base_url.clone(),
            api_key: config.api_key.clone(),
        }
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
}
