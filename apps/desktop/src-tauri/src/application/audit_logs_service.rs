use std::sync::Arc;

use crate::domain::entities::AuditLog;
use crate::domain::ports::AuditLogRepository;

pub struct AuditLogsService {
    repo: Arc<dyn AuditLogRepository>,
}

impl AuditLogsService {
    pub fn new(repo: Arc<dyn AuditLogRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_audit_logs(
        &self,
        guild_id: Option<String>,
        event_type: Option<String>,
        limit: Option<i64>,
    ) -> Result<Vec<AuditLog>, String> {
        self.repo.get_audit_logs(guild_id, event_type, limit).await
    }
}
