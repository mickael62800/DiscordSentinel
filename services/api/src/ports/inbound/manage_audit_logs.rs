use async_trait::async_trait;

use crate::domain::entities::AuditLog;
use crate::domain::errors::DomainError;

pub struct CreateAuditLogCommand {
    pub guild_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub details: serde_json::Value,
}

pub struct AuditLogFilters {
    pub event_type: Option<String>,
    pub actor_id: Option<String>,
    pub target_id: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

#[async_trait]
pub trait ManageAuditLogsUseCase: Send + Sync {
    async fn create(&self, command: CreateAuditLogCommand) -> Result<AuditLog, DomainError>;
    async fn list(&self, guild_id: Option<&str>, filters: AuditLogFilters) -> Result<Vec<AuditLog>, DomainError>;

    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError>;
}
