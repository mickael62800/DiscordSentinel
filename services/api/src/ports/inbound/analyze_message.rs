use async_trait::async_trait;

use crate::domain::entities::MessageAnalysis;
use crate::domain::errors::DomainError;
use crate::domain::value_objects::DetectionFlags;

/// Entree de contexte conversationnel (message precedent dans le canal).
pub struct ContextMessageEntry {
    pub username: String,
    pub content: String,
}

#[allow(dead_code)]
pub struct AnalyzeMessageCommand {
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub content: String,
    pub flags: DetectionFlags,
    pub message_id: String,
    pub timestamp: String,
    /// Messages de contexte conversationnel pour l'analyse de sentiment.
    pub context_messages: Vec<ContextMessageEntry>,
}

#[async_trait]
pub trait AnalyzeMessageUseCase: Send + Sync {
    async fn analyze(&self, command: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError>;
}
