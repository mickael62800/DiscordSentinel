use std::sync::Arc;

use serde::{Deserialize, Serialize};

use sentinel_shared::api_client::BaseApiClient;

use crate::detectors::DetectionFlags;

/// Payload envoye au backend pour analyse.
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

/// Reponse du backend.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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

/// Client specifique a l'automod-bot, encapsule le BaseApiClient partage.
pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    /// Envoie un message au backend pour analyse et retourne l'action a effectuer.
    /// Timeout de 5 secondes pour eviter de bloquer le bot.
    /// Note: uses custom timeout, cannot use BaseApiClient helpers.
    pub async fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzeResponse, String> {
        let req = self
            .base
            .client()
            .post(format!("{}/analyze", self.base.base_url()))
            .timeout(std::time::Duration::from_secs(5))
            .json(request);

        let resp = self.base.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Echec lecture body de la reponse API");
                String::new()
            });
            return Err(format!("API error {}: {}", status, body));
        }

        resp.json::<AnalyzeResponse>()
            .await
            .map_err(|e| format!("Parse error: {e}"))
    }
}
