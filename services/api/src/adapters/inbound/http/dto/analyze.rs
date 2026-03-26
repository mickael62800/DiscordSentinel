use serde::{Deserialize, Serialize};

use crate::domain::entities::MessageAnalysis;
use crate::domain::value_objects::DetectionFlags;
use crate::ports::inbound::AnalyzeMessageCommand;

/// DTO de la requête reçue depuis le bot automod.
#[derive(Debug, Deserialize)]
pub struct AnalyzeRequestDto {
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub content: String,
    pub flags: DetectionFlags,
    pub metadata: MetadataDto,
}

#[derive(Debug, Deserialize)]
pub struct MetadataDto {
    pub message_id: String,
    pub timestamp: String,
}

/// DTO de la réponse renvoyée au bot.
#[derive(Debug, Serialize)]
pub struct AnalyzeResponseDto {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

impl From<AnalyzeRequestDto> for AnalyzeMessageCommand {
    fn from(dto: AnalyzeRequestDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            channel_id: dto.channel_id,
            user_id: dto.user_id,
            username: dto.username,
            content: dto.content,
            flags: dto.flags,
            message_id: dto.metadata.message_id,
            timestamp: dto.metadata.timestamp,
        }
    }
}

impl From<MessageAnalysis> for AnalyzeResponseDto {
    fn from(analysis: MessageAnalysis) -> Self {
        Self {
            action: analysis.action.as_str().to_string(),
            reason: if analysis.reason.is_empty() {
                None
            } else {
                Some(analysis.reason)
            },
            duration: analysis.duration,
        }
    }
}
