use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::{level_from_xp, LevelConfig, LevelReward, UserLevel, XpSource};
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

        let (old_xp, old_xp_text, old_xp_voice, id) = match &existing {
            Some(u) => (u.xp, u.xp_text, u.xp_voice, u.id),
            None => (0, 0, 0, Uuid::new_v4()),
        };

        // Ajouter l'XP a la source correspondante ET au total
        let (new_xp_text, new_xp_voice) = match cmd.source {
            XpSource::Text => (old_xp_text + cmd.amount, old_xp_voice),
            XpSource::Voice => (old_xp_text, old_xp_voice + cmd.amount),
        };
        let new_xp = old_xp + cmd.amount;

        let new_level = level_from_xp(new_xp);
        let new_level_text = level_from_xp(new_xp_text);
        let new_level_voice = level_from_xp(new_xp_voice);

        // Detecter le level-up de la source specifique
        let old_source_level = match cmd.source {
            XpSource::Text => existing.as_ref().map(|u| u.level_text).unwrap_or(0),
            XpSource::Voice => existing.as_ref().map(|u| u.level_voice).unwrap_or(0),
        };
        let new_source_level = match cmd.source {
            XpSource::Text => new_level_text,
            XpSource::Voice => new_level_voice,
        };

        let user_level = UserLevel {
            id,
            guild_id: cmd.guild_id.clone(),
            user_id: cmd.user_id.clone(),
            username: cmd.username,
            xp: new_xp,
            level: new_level,
            xp_text: new_xp_text,
            level_text: new_level_text,
            xp_voice: new_xp_voice,
            level_voice: new_level_voice,
            last_xp_at: now,
            created_at: existing.map(|u| u.created_at).unwrap_or(now),
            updated_at: now,
        };

        self.repo.upsert_user_level(&user_level).await?;

        let leveled_up = new_source_level > old_source_level;
        let reward_role_id = if leveled_up {
            let rewards = self.repo.get_rewards_by_source(&cmd.guild_id, cmd.source).await?;
            rewards
                .iter()
                .find(|r| r.level == new_source_level)
                .map(|r| r.role_id.clone())
        } else {
            None
        };

        Ok(AddXpResult {
            user_level,
            leveled_up,
            old_level: old_source_level,
            reward_role_id,
            source: cmd.source,
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

    async fn get_leaderboard_by_source(&self, guild_id: &str, source: XpSource, limit: i64) -> Result<Vec<UserLevel>, DomainError> {
        self.repo.get_leaderboard_by_source(guild_id, source, limit).await
    }

    async fn get_rewards(&self, guild_id: &str) -> Result<Vec<LevelReward>, DomainError> {
        self.repo.get_rewards(guild_id).await
    }

    async fn get_rewards_by_source(&self, guild_id: &str, source: XpSource) -> Result<Vec<LevelReward>, DomainError> {
        self.repo.get_rewards_by_source(guild_id, source).await
    }

    async fn set_reward(&self, guild_id: &str, level: i32, role_id: &str, source: XpSource) -> Result<LevelReward, DomainError> {
        let reward = LevelReward {
            id: Uuid::new_v4(),
            guild_id: guild_id.to_string(),
            level,
            role_id: role_id.to_string(),
            source,
        };
        self.repo.upsert_reward(&reward).await?;
        Ok(reward)
    }

    async fn delete_reward(&self, guild_id: &str, level: i32, source: XpSource) -> Result<(), DomainError> {
        self.repo.delete_reward(guild_id, level, source).await
    }
}
