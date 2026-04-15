use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use tracing::warn;

use crate::domain::entities::SecurityEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::{
    CreateAuditLogCommand, ManageAuditLogsUseCase, ManageSecurityUseCase, ReportSecurityEventCommand,
};
use crate::ports::outbound::{CachePort, SecurityEventRepository, WatchedUserRepository};

const EVENTS_TTL: u64 = 60; // 1 minute

pub struct ManageSecurityService {
    repo: Arc<dyn SecurityEventRepository>,
    cache: Arc<dyn CachePort>,
    watched_repo: Arc<dyn WatchedUserRepository>,
    audit_logs_uc: Option<Arc<dyn ManageAuditLogsUseCase>>,
}

impl ManageSecurityService {
    pub fn new(
        repo: Arc<dyn SecurityEventRepository>,
        cache: Arc<dyn CachePort>,
        watched_repo: Arc<dyn WatchedUserRepository>,
    ) -> Self {
        Self { repo, cache, watched_repo, audit_logs_uc: None }
    }

    /// Phase 1 dual-write : copie chaque evenement de securite dans audit_logs
    /// avec event_type `security_<event_type>`.
    pub fn with_audit_logs_uc(mut self, audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>) -> Self {
        self.audit_logs_uc = Some(audit_logs_uc);
        self
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

        // Phase 4 : repo.save() est un no-op. La persistence est portee par
        // audit_logs_uc.create. Erreur dure si non injecte.
        self.repo.save(&event).await?;

        let uc = self.audit_logs_uc.as_ref().ok_or_else(|| {
            DomainError::Internal(
                "audit_logs_uc non injecte dans ManageSecurityService".into(),
            )
        })?;
        let event_type_str = format!("security_{}", event.event_type);
        let details = serde_json::json!({
            "severity": event.severity,
            "description": event.description,
            "user_ids": event.user_ids,
            "event_id": event.id.to_string(),
        });
        let (target_id, target_name) = match event.user_ids.as_slice() {
            [single] => (Some(single.clone()), Some(single.clone())),
            _ => (None, None),
        };
        let cmd = CreateAuditLogCommand {
            guild_id: event.guild_id.clone(),
            event_type: event_type_str,
            actor_id: None,
            actor_name: None,
            target_id,
            target_name,
            channel_id: None,
            channel_name: None,
            details,
        };
        uc.create(cmd).await?;

        // Auto-surveillance : place chaque user concerne en manual watch.
        for uid in &event.user_ids {
            let reason = format!("Auto: {} ({})", event.event_type, event.severity);
            if let Err(e) = self
                .watched_repo
                .add_manual_watch(&event.guild_id, uid, uid, &reason, "security_event")
                .await
            {
                warn!(error = %e, guild_id = %event.guild_id, user_id = %uid, "Echec auto-surveillance");
            }
        }

        // Invalidate events cache
        if let Err(e) = self.cache.invalidate("security:all").await {
            warn!(error = %e, "Echec invalidation cache security:all");
        }
        if let Err(e) = self.cache.invalidate(&format!("security:{}", event.guild_id)).await {
            warn!(error = %e, guild_id = %event.guild_id, "Echec invalidation cache security guild");
        }

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
            if let Err(e) = self.cache.set_json(&cache_key, &json, EVENTS_TTL).await {
                warn!(error = %e, cache_key = %cache_key, "Echec cache set security events");
            }
        }

        Ok(events)
    }
}
