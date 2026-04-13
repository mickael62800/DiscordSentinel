//! Client API du image-bot.
//!
//! Phase 7A — Migration gRPC complete :
//! - `analyze_image` -> `ImagesService.AnalyzeImage` avec **bytes natifs**.
//!   Plus de base64 (gain ~33% sur la bande passante vs HTTP+JSON pour des
//!   images de plusieurs centaines de Ko, ce qui est typique en Discord).
//! - `download_image` reste sur le client HTTP brut (telechargement
//!   d'attachments Discord externes, pas de l'API Sentinel).
//!
//! Surface publique : `image_data` du `AnalyzeImageRequest` est maintenant
//! `Vec<u8>` au lieu de `String` base64. Le handler ne fait plus l'encodage.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

use sentinel_proto::images::v1 as proto;

#[derive(Debug, Serialize)]
pub struct AnalyzeImageRequest {
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub message_id: String,
    /// Bytes natifs de l'image — plus de base64.
    #[serde(skip)]
    pub image_data: Vec<u8>,
    pub content_type: String,
    pub filename: String,
    /// Conserves pour compat handler — pas envoyes en proto v1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_override: Option<f64>,
    #[serde(default)]
    pub is_screenshot: bool,
    #[serde(default)]
    pub is_animated: bool,
}

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

pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
    grpc: Arc<SentinelGrpcClient>,
    max_image_size: u64,
}

impl ApiClient {
    pub fn new(
        base: Arc<BaseApiClient>,
        grpc: Arc<SentinelGrpcClient>,
        max_image_size: u64,
    ) -> Self {
        Self {
            base,
            grpc,
            max_image_size,
        }
    }

    pub fn max_image_size(&self) -> u64 {
        self.max_image_size
    }

    /// gRPC `ImagesService.AnalyzeImage`. Bytes natifs, pas de base64.
    pub async fn analyze_image(
        &self,
        request: &AnalyzeImageRequest,
    ) -> Result<AnalyzeImageResponse, String> {
        let req = proto::AnalyzeImageRequest {
            guild_id: request.guild_id.clone(),
            channel_id: request.channel_id.clone(),
            user_id: request.user_id.clone(),
            username: request.username.clone(),
            message_id: request.message_id.clone(),
            image_data: request.image_data.clone(),
            content_type: request.content_type.clone(),
            filename: request.filename.clone(),
        };
        let mut client = self.grpc.images();
        let resp = self
            .grpc
            .guarded(|| async move {
                client.analyze_image(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(AnalyzeImageResponse {
            action: proto_action_to_action(resp.action),
            reason: if resp.reason.is_empty() {
                None
            } else {
                Some(resp.reason)
            },
            duration: resp.duration,
            classifications: resp
                .classifications
                .into_iter()
                .map(|c| Classification {
                    label: c.label,
                    confidence: c.confidence,
                })
                .collect(),
        })
    }

    /// Telecharge une image depuis une URL externe (attachment Discord).
    /// Reste sur HTTP brut — ce n'est pas un appel a l'API Sentinel.
    ///
    /// Garde-fous defense-in-depth :
    /// - timeout global de 30s (evite qu'un serveur malveillant bloque
    ///   indefiniment la task tokio)
    /// - rejet avant download si Content-Length > max_image_size (evite
    ///   de commencer un transfert enorme)
    /// - rejet apres download si la taille reelle depasse max_image_size
    ///   (les serveurs peuvent mentir sur Content-Length)
    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>, String> {
        use std::time::Duration;

        let fut = async {
            let resp = self
                .base
                .client()
                .get(url)
                .send()
                .await
                .map_err(|e| format!("download: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("download: HTTP {}", resp.status()));
            }

            if let Some(len) = resp.content_length() {
                if len > self.max_image_size {
                    return Err(format!(
                        "download: fichier trop gros ({} > {} max)",
                        len, self.max_image_size
                    ));
                }
            }

            let bytes = resp.bytes().await.map_err(|e| format!("download body: {e}"))?;
            if bytes.len() as u64 > self.max_image_size {
                return Err(format!(
                    "download: fichier depasse la taille max ({} bytes)",
                    self.max_image_size
                ));
            }
            Ok(bytes.to_vec())
        };

        match tokio::time::timeout(Duration::from_secs(30), fut).await {
            Ok(r) => r,
            Err(_) => Err("download: timeout apres 30s".to_string()),
        }
    }
}

fn proto_action_to_action(value: i32) -> Action {
    match proto::Action::try_from(value).unwrap_or(proto::Action::None) {
        proto::Action::None => Action::None,
        proto::Action::Warn => Action::Warn,
        proto::Action::Delete => Action::Delete,
        proto::Action::Mute => Action::Mute,
        proto::Action::Ban => Action::Ban,
    }
}

fn grpc_err_to_string(e: GrpcCallError) -> String {
    match e {
        GrpcCallError::Unavailable => "API indisponible (circuit breaker ouvert)".to_string(),
        GrpcCallError::Status(s) => format!("gRPC {:?}: {}", s.code(), s.message()),
        GrpcCallError::Transport(t) => format!("transport gRPC: {t}"),
    }
}
