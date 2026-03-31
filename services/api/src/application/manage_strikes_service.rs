use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::domain::entities::{StrikeConfig, StrikeResult, UserStrike};
use crate::domain::errors::DomainError;
use crate::ports::inbound::{AddStrikeCommand, ManageStrikesUseCase, SaveStrikeConfigCommand};
use crate::ports::outbound::StrikeRepository;

pub struct ManageStrikesService {
    repo: Arc<dyn StrikeRepository>,
}

impl ManageStrikesService {
    pub fn new(repo: Arc<dyn StrikeRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageStrikesUseCase for ManageStrikesService {
    async fn add_strike(&self, cmd: AddStrikeCommand) -> Result<StrikeResult, DomainError> {
        let config = self.get_config(&cmd.guild_id).await?;

        let expires_at = if config.window_secs > 0 {
            Some(Utc::now() + Duration::seconds(config.window_secs))
        } else {
            None
        };

        let strike = UserStrike {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            user_id: cmd.user_id.clone(),
            reason: cmd.reason,
            source: cmd.source,
            infraction_id: cmd.infraction_id,
            expires_at,
            created_at: Utc::now(),
        };

        self.repo.save_strike(&strike).await?;

        if !config.enabled {
            return Ok(StrikeResult {
                strike,
                active_count: 1,
                escalation_action: None,
                escalation_duration: None,
            });
        }

        let active = self.repo.find_active_strikes(&cmd.guild_id, &cmd.user_id, config.window_secs).await?;
        let active_count = active.len() as u32;

        let mut sorted_thresholds = config.thresholds.clone();
        sorted_thresholds.sort_by(|a, b| b.strikes.cmp(&a.strikes));

        let escalation = sorted_thresholds
            .iter()
            .find(|t| active_count >= t.strikes);

        let (escalation_action, escalation_duration) = match escalation {
            Some(t) => (Some(t.action.clone()), t.duration),
            None => (None, None),
        };

        Ok(StrikeResult {
            strike,
            active_count,
            escalation_action,
            escalation_duration,
        })
    }

    async fn get_active_strikes(&self, guild_id: &str, user_id: &str) -> Result<Vec<UserStrike>, DomainError> {
        let config = self.get_config(guild_id).await?;
        self.repo.find_active_strikes(guild_id, user_id, config.window_secs).await
    }

    async fn reset_strikes(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.repo.delete_strikes(guild_id, user_id).await
    }

    async fn get_config(&self, guild_id: &str) -> Result<StrikeConfig, DomainError> {
        match self.repo.get_config(guild_id).await? {
            Some(config) => Ok(config),
            None => Ok(StrikeConfig::default_for_guild(guild_id)),
        }
    }

    async fn save_config(&self, cmd: SaveStrikeConfigCommand) -> Result<StrikeConfig, DomainError> {
        let config = StrikeConfig {
            guild_id: cmd.guild_id,
            window_secs: cmd.window_secs,
            thresholds: cmd.thresholds,
            enabled: cmd.enabled,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.repo.save_config(&config).await?;
        Ok(config)
    }
}
