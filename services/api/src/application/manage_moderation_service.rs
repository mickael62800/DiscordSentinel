use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{ModerationAction, UserModerationHistory};
use crate::domain::errors::DomainError;
use crate::ports::inbound::{DeductPointsCommand, LogModerationCommand, ManageConductUseCase, ManageModerationUseCase};
use crate::ports::outbound::{CachePort, ModerationRepository};

const HISTORY_TTL: u64 = 180; // 3 minutes

pub struct ManageModerationService {
    repo: Arc<dyn ModerationRepository>,
    cache: Arc<dyn CachePort>,
    conduct_uc: Arc<dyn ManageConductUseCase>,
}

impl ManageModerationService {
    pub fn new(repo: Arc<dyn ModerationRepository>, cache: Arc<dyn CachePort>, conduct_uc: Arc<dyn ManageConductUseCase>) -> Self {
        Self { repo, cache, conduct_uc }
    }
}

#[async_trait]
impl ManageModerationUseCase for ManageModerationService {
    async fn log_action(&self, cmd: LogModerationCommand) -> Result<ModerationAction, DomainError> {
        let action = ModerationAction {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            channel_id: cmd.channel_id,
            moderator_id: cmd.moderator_id,
            moderator_name: cmd.moderator_name,
            target_id: cmd.target_id.clone(),
            target_name: cmd.target_name,
            action_type: cmd.action_type,
            reason: cmd.reason,
            gravity: cmd.gravity,
            duration: cmd.duration,
            created_at: chrono::Utc::now(),
        };

        self.repo.save(&action).await?;

        // Invalidate history cache for this user
        let cache_key = format!("modhistory:{}:{}", cmd.guild_id, cmd.target_id);
        self.cache.invalidate(&cache_key).await.ok();

        // Deduire les points de conduite
        let _ = self.conduct_uc.deduct_points(DeductPointsCommand {
            guild_id: action.guild_id.clone(),
            user_id: action.target_id.clone(),
            username: action.target_name.clone(),
            action: action.action_type.clone(),
        }).await;

        Ok(action)
    }

    async fn get_history(&self, guild_id: &str, target_id: &str) -> Result<UserModerationHistory, DomainError> {
        let cache_key = format!("modhistory:{guild_id}:{target_id}");

        // Cache-first
        if let Some(json) = self.cache.get_json(&cache_key).await? {
            if let Ok(history) = serde_json::from_str::<UserModerationHistory>(&json) {
                return Ok(history);
            }
        }

        let actions = self.repo.find_by_target(guild_id, target_id).await?;
        let target_name = actions.first().map(|a| a.target_name.clone()).unwrap_or_default();

        let total_warns = actions.iter().filter(|a| a.action_type == "warn").count() as u32;
        let total_mutes = actions.iter().filter(|a| a.action_type.starts_with("mute")).count() as u32;
        let total_bans = actions.iter().filter(|a| a.action_type.starts_with("ban")).count() as u32;

        let history = UserModerationHistory {
            target_id: target_id.to_string(),
            target_name,
            total_warns,
            total_mutes,
            total_bans,
            actions,
        };

        // Populate cache
        if let Ok(json) = serde_json::to_string(&history) {
            self.cache.set_json(&cache_key, &json, HISTORY_TTL).await.ok();
        }

        Ok(history)
    }

    async fn list_bans(&self, guild_id: Option<&str>) -> Result<Vec<ModerationAction>, DomainError> {
        self.repo.find_bans(guild_id).await
    }

    async fn delete_bans_for_user(&self, guild_id: &str, target_id: &str) -> Result<(), DomainError> {
        self.repo.delete_bans_for_user(guild_id, target_id).await
    }
}
