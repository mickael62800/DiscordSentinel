use std::sync::Arc;

use serde::{Deserialize, Serialize};

use sentinel_shared::api_client::BaseApiClient;

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
    /// Seuil de confiance override pour ce salon (optionnel)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_override: Option<f64>,
    /// True si l'image est un screenshot (pour OCR cote API)
    #[serde(default)]
    pub is_screenshot: bool,
    /// True si l'image est un GIF anime
    #[serde(default)]
    pub is_animated: bool,
}

/// Reponse du backend apres analyse d'image.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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
#[allow(dead_code)]
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

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::None => "none",
            Action::Warn => "warn",
            Action::Delete => "delete",
            Action::Mute => "mute",
            Action::Ban => "ban",
        }
    }
}

/// Client specifique a l'image-bot, encapsule le BaseApiClient partage.
pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
    max_image_size: u64,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>, max_image_size: u64) -> Self {
        Self {
            base,
            max_image_size,
        }
    }

    pub fn max_image_size(&self) -> u64 {
        self.max_image_size
    }

    /// Envoie une image au backend pour analyse (NSFW / produits illicites).
    /// Note: returns reqwest::Error and uses raw client — cannot use BaseApiClient helpers.
    pub async fn analyze_image(
        &self,
        request: &AnalyzeImageRequest,
    ) -> Result<AnalyzeImageResponse, reqwest::Error> {
        let req = self
            .base
            .client()
            .post(format!("{}/analyze/image", self.base.base_url()))
            .json(request);

        self.base.auth(req).send().await?.json().await
    }

    /// Telecharge une image depuis une URL (attachment Discord).
    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>, reqwest::Error> {
        let bytes = self.base.client().get(url).send().await?.bytes().await?;
        Ok(bytes.to_vec())
    }
}
