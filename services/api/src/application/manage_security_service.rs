use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::SecurityEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::{ManageSecurityUseCase, ReportSecurityEventCommand};
use crate::ports::outbound::SecurityEventRepository;

pub struct ManageSecurityService {
    repo: Arc<dyn SecurityEventRepository>,
}

impl ManageSecurityService {
    pub fn new(repo: Arc<dyn SecurityEventRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageSecurityUseCase for ManageSecurityService {
    async fn report_event(
        &self,
        cmd: ReportSecurityEventCommand,
    ) -> Result<SecurityEvent, DomainError> {
        let event = SecurityEvent {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            event_type: cmd.event_type,
            severity: cmd.severity,
            description: cmd.description,
            user_ids: cmd.user_ids,
            created_at: chrono::Utc::now(),
        };

        self.repo.save(&event).await?;

        Ok(event)
    }

    async fn list_events(&self, guild_id: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError> {
        match guild_id {
            Some(gid) => self.repo.find_by_guild(gid).await,
            None => self.repo.find_all().await,
        }
    }
}
