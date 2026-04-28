use async_trait::async_trait;

use crate::domain::entities::ImageAnalysis;
use crate::domain::errors::DomainError;

#[allow(dead_code)]
pub struct AnalyzeImageCommand {
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub message_id: String,
    /// Image brute en bytes (decodee depuis base64 par le handler)
    pub image_bytes: Vec<u8>,
    pub content_type: String,
    pub filename: String,
}

#[async_trait]
pub trait AnalyzeImageUseCase: Send + Sync {
    async fn analyze_image(&self, command: AnalyzeImageCommand) -> Result<ImageAnalysis, DomainError>;
}
