use async_trait::async_trait;

use crate::domain::entities::audit::audit_log::AuditLog;
use crate::domain::errors::DomainError;
use crate::ports::inbound::audit::manage_audit_logs::AuditLogFilters;

#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    async fn save(&self, log: &AuditLog) -> Result<(), DomainError>;
    async fn find_all(&self, guild_id: Option<&str>, filters: &AuditLogFilters) -> Result<Vec<AuditLog>, DomainError>;
    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError>;
}
