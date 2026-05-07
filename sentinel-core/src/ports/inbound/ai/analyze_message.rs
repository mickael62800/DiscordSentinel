use async_trait::async_trait;

use crate::domain::entities::ai::message_analysis::MessageAnalysis;
use crate::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::domain::entities::system::discord_ids::ChannelId;
use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::entities::system::discord_ids::MessageId;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::errors::DomainError;

/// Entree de contexte conversationnel (message precedent dans le canal).
pub struct ContextMessageEntry {
    pub username: String,
    pub content: String,
}

#[allow(dead_code)]
pub struct AnalyzeMessageCommand {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub username: String,
    pub content: String,
    pub flags: DetectionFlags,
    pub message_id: MessageId,
    pub timestamp: String,
    /// Messages de contexte conversationnel pour l'analyse de sentiment.
    pub context_messages: Vec<ContextMessageEntry>,
}

#[async_trait]
pub trait AnalyzeMessageUseCase: Send + Sync {
    async fn analyze(&self, command: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError>;
}
