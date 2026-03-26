use async_trait::async_trait;

use crate::domain::entities::MessageAnalysis;
use crate::domain::errors::DomainError;
use crate::domain::value_objects::DetectionFlags;

pub struct AnalyzeMessageCommand {
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub content: String,
    pub flags: DetectionFlags,
    pub message_id: String,
    pub timestamp: String,
}

#[async_trait]
pub trait AnalyzeMessageUseCase: Send + Sync {
    async fn analyze(&self, command: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError>;
}
