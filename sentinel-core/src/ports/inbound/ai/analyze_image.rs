use async_trait::async_trait;

use crate::domain::entities::ai::image_analysis::ImageAnalysis;
use crate::domain::entities::system::discord_ids::ChannelId;
use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::entities::system::discord_ids::MessageId;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::errors::DomainError;

#[allow(dead_code)]
pub struct AnalyzeImageCommand {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub username: String,
    pub message_id: MessageId,
    /// Image brute en bytes (decodee depuis base64 par le handler)
    pub image_bytes: Vec<u8>,
    pub content_type: String,
    pub filename: String,
}

#[async_trait]
pub trait AnalyzeImageUseCase: Send + Sync {
    async fn analyze_image(
        &self,
        command: AnalyzeImageCommand,
    ) -> Result<ImageAnalysis, DomainError>;
}
