use async_trait::async_trait;

use crate::domain::entities::SecurityEvent;
use crate::domain::errors::DomainError;

pub struct ReportSecurityEventCommand {
    pub guild_id: String,
    pub event_type: String,
    pub severity: String,
    pub description: String,
    pub user_ids: Vec<String>,
}

#[async_trait]
pub trait ManageSecurityUseCase: Send + Sync {
    async fn report_event(
        &self,
        command: ReportSecurityEventCommand,
    ) -> Result<SecurityEvent, DomainError>;
    async fn list_events(&self, guild_id: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError>;
}
