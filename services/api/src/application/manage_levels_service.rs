use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::{level_from_xp, LevelConfig, LevelReward, UserLevel};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_levels::{
    AddXpCommand, AddXpResult, ManageLevelsUseCase, SaveLevelConfigCommand,
};
use crate::ports::outbound::LevelRepository;

pub struct ManageLevelsService {
    repo: Arc<dyn LevelRepository>,
}

impl ManageLevelsService {
    pub fn new(repo: Arc<dyn LevelRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageLevelsUseCase for ManageLevelsService {
    async fn get_config(&self, guild_id: &str) -> Result<LevelConfig, DomainError> {
        self.repo
            .get_config(guild_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Config niveaux introuvable pour {guild_id}")))
    }

    async fn save_config(&self, cmd: SaveLevelConfigCommand) -> Result<LevelConfig, DomainError> {
        let now = Utc::now();
        let config = LevelConfig {
            guild_id: cmd.guild_id,
            xp_per_message: cmd.xp_per_message,
            xp_per_voice_minute: cmd.xp_per_voice_minute,
            xp_cooldown_secs: cmd.xp_cooldown_secs,
            level_up_channel_id: cmd.level_up_channel_id,
            level_up_message: cmd.level_up_message,
            excluded_channels: cmd.excluded_channels,
            enabled: cmd.enabled,
            created_at: now,
            updated_at: now,
        };
        self.repo.upsert_config(&config).await?;
        Ok(config)
    }

    async fn add_xp(&self, cmd: AddXpCommand) -> Result<AddXpResult, DomainError> {
        let now = Utc::now();
        let existing = self.repo.get_user_level(&cmd.guild_id, &cmd.user_id).await?;

        let (old_xp, old_level, id) = match &existing {
            Some(u) => (u.xp, u.level, u.id),
            None => (0, 0, Uuid::new_v4()),
        };

        let new_xp = old_xp + cmd.amount;
        let new_level = level_from_xp(new_xp);

        let user_level = UserLevel {
            id,
            guild_id: cmd.guild_id.clone(),
            user_id: cmd.user_id.clone(),
            username: cmd.username,
            xp: new_xp,
            level: new_level,
            last_xp_at: now,
            created_at: existing.map(|u| u.created_at).unwrap_or(now),
            updated_at: now,
        };

        self.repo.upsert_user_level(&user_level).await?;

        let leveled_up = new_level > old_level;
        let reward_role_id = if leveled_up {
            let rewards = self.repo.get_rewards(&cmd.guild_id).await?;
            rewards
                .iter()
                .find(|r| r.level == new_level)
                .map(|r| r.role_id.clone())
        } else {
            None
        };

        Ok(AddXpResult {
            user_level,
            leveled_up,
            old_level,
            reward_role_id,
        })
    }

    async fn get_user_level(&self, guild_id: &str, user_id: &str) -> Result<UserLevel, DomainError> {
        self.repo
            .get_user_level(guild_id, user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Niveau introuvable pour {user_id}")))
    }

    async fn get_leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<UserLevel>, DomainError> {
        self.repo.get_leaderboard(guild_id, limit).await
    }

    async fn get_rewards(&self, guild_id: &str) -> Result<Vec<LevelReward>, DomainError> {
        self.repo.get_rewards(guild_id).await
    }

    async fn set_reward(&self, guild_id: &str, level: i32, role_id: &str) -> Result<LevelReward, DomainError> {
        let reward = LevelReward {
            id: Uuid::new_v4(),
            guild_id: guild_id.to_string(),
            level,
            role_id: role_id.to_string(),
        };
        self.repo.upsert_reward(&reward).await?;
        Ok(reward)
    }

    async fn delete_reward(&self, guild_id: &str, level: i32) -> Result<(), DomainError> {
        self.repo.delete_reward(guild_id, level).await
    }
}
