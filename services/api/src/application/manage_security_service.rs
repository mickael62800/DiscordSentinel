use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::SecurityEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::{ManageSecurityUseCase, ReportSecurityEventCommand};
use crate::ports::outbound::{CachePort, SecurityEventRepository};

const EVENTS_TTL: u64 = 60; // 1 minute

pub struct ManageSecurityService {
    repo: Arc<dyn SecurityEventRepository>,
    cache: Arc<dyn CachePort>,
}

impl ManageSecurityService {
    pub fn new(repo: Arc<dyn SecurityEventRepository>, cache: Arc<dyn CachePort>) -> Self {
        Self { repo, cache }
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

        // Invalidate events cache
        self.cache.invalidate("security:all").await.ok();
        self.cache.invalidate(&format!("security:{}", event.guild_id)).await.ok();

        Ok(event)
    }

    async fn list_events(&self, guild_id: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError> {
        let cache_key = match guild_id {
            Some(gid) => format!("security:{gid}"),
            None => "security:all".to_string(),
        };

        // Cache-first
        if let Some(json) = self.cache.get_json(&cache_key).await? {
            if let Ok(events) = serde_json::from_str::<Vec<SecurityEvent>>(&json) {
                return Ok(events);
            }
        }

        let events = match guild_id {
            Some(gid) => self.repo.find_by_guild(gid).await?,
            None => self.repo.find_all().await?,
        };

        // Populate cache
        if let Ok(json) = serde_json::to_string(&events) {
            self.cache.set_json(&cache_key, &json, EVENTS_TTL).await.ok();
        }

        Ok(events)
    }
}
