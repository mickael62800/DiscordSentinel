use async_trait::async_trait;

use sentinel_core::domain::entities::ai::image_analysis::ImageAnalysis;
use sentinel_core::domain::entities::system::discord_ids::ChannelId;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
use sentinel_core::domain::entities::system::discord_ids::MessageId;
use sentinel_core::domain::entities::system::discord_ids::UserId;
use sentinel_core::domain::errors::DomainError;

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
    async fn analyze_image(&self, command: AnalyzeImageCommand) -> Result<ImageAnalysis, DomainError>;
}
