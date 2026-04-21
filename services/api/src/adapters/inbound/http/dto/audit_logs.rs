use serde::{Deserialize, Serialize};

use crate::domain::entities::AuditLog;
use crate::ports::inbound::manage_audit_logs::CreateAuditLogCommand;

#[derive(Debug, Deserialize)]
pub struct CreateAuditLogDto {
    pub guild_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    #[serde(default = "default_details")]
    pub details: serde_json::Value,
}

fn default_details() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQueryParams {
    pub guild_id: Option<String>,
    pub event_type: Option<String>,
    pub actor_id: Option<String>,
    pub target_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponseDto {
    pub id: String,
    pub guild_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub details: serde_json::Value,
    pub created_at: String,
}

impl From<CreateAuditLogDto> for CreateAuditLogCommand {
    fn from(dto: CreateAuditLogDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            event_type: dto.event_type,
            actor_id: dto.actor_id,
            actor_name: dto.actor_name,
            target_id: dto.target_id,
            target_name: dto.target_name,
            channel_id: dto.channel_id,
            channel_name: dto.channel_name,
            details: dto.details,
        }
    }
}

impl From<AuditLog> for AuditLogResponseDto {
    fn from(log: AuditLog) -> Self {
        Self {
            id: log.id.to_string(),
            guild_id: log.guild_id,
            event_type: log.event_type,
            actor_id: log.actor_id,
            actor_name: log.actor_name,
            target_id: log.target_id,
            target_name: log.target_name,
            channel_id: log.channel_id,
            channel_name: log.channel_name,
            details: log.details,
            created_at: log.created_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
#[path = "tests/audit_logs.rs"]
mod tests;
