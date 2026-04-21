use serde::{Deserialize, Serialize};

use crate::domain::entities::SecurityEvent;
use crate::ports::inbound::ReportSecurityEventCommand;

#[derive(Debug, Deserialize)]
pub struct ReportEventDto {
    pub guild_id: String,
    pub event_type: String,
    pub severity: String,
    pub description: String,
    #[serde(default)]
    pub user_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SecurityEventResponseDto {
    pub id: String,
    pub guild_id: String,
    pub event_type: String,
    pub severity: String,
    pub description: String,
    pub user_ids: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SecurityQueryParams {
    pub guild_id: Option<String>,
}

impl From<ReportEventDto> for ReportSecurityEventCommand {
    fn from(dto: ReportEventDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            event_type: dto.event_type,
            severity: dto.severity,
            description: dto.description,
            user_ids: dto.user_ids,
        }
    }
}

impl From<SecurityEvent> for SecurityEventResponseDto {
    fn from(e: SecurityEvent) -> Self {
        Self {
            id: e.id.to_string(),
            guild_id: e.guild_id,
            event_type: e.event_type,
            severity: e.severity,
            description: e.description,
            user_ids: e.user_ids,
            created_at: e.created_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
#[path = "tests/security.rs"]
mod tests;
