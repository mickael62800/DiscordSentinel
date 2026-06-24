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

/// Decision d'auto-protection face a un flood, prise cote serveur a partir
/// de la config guild (`auto_protect_enabled`, `severe_flood_max_messages`).
/// Le bot detecte le flood (tracker rate en memoire, legitime) puis demande
/// le verdict ici au lieu de comparer a un seuil local.
pub struct FloodDecision {
    /// True si une protection automatique (mute + suppression) doit s'appliquer.
    pub severe: bool,
    /// Duree du mute a appliquer si `severe` (secondes).
    pub mute_duration_secs: i64,
}

#[async_trait]
pub trait AnalyzeMessageUseCase: Send + Sync {
    async fn analyze(&self, command: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError>;

    /// Evalue un signal de flood (nombre de messages dans la fenetre) et
    /// renvoie la decision d'auto-protection. La regle (seuil severe, toggle)
    /// vit cote serveur, pas dans le bot.
    async fn evaluate_flood(
        &self,
        guild_id: &str,
        flood_count: i32,
    ) -> Result<FloodDecision, DomainError>;
}
