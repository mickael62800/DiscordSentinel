use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use sentinel_core::domain::entities::community::level::level_from_xp;
use sentinel_core::domain::entities::community::level::LevelConfig;
use sentinel_core::domain::entities::community::level::LevelReward;
use sentinel_core::domain::entities::community::level::UserLevel;
use sentinel_core::domain::entities::community::level::XpSource;
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::community::manage_levels::AddXpCommand;
use crate::ports::inbound::community::manage_levels::AddXpResult;
use crate::ports::inbound::community::manage_levels::ManageLevelsUseCase;
use crate::ports::inbound::community::manage_levels::ResetTarget;
use crate::ports::inbound::community::manage_levels::SaveLevelConfigCommand;
use crate::ports::inbound::community::manage_levels::SetUserXpCommand;
use crate::ports::outbound::community::level_repository::LevelRepository;

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
        // Si la guild n'a jamais sauve de config, on retourne les defauts au
        // lieu d'un 404 — sinon le frontend pollue la console et l'admin est
        // perdu pour comprendre comment "creer" la config initiale.
        if let Some(cfg) = self.repo.get_config(guild_id).await? {
            return Ok(cfg);
        }
        let now = Utc::now();
        Ok(LevelConfig {
            guild_id: guild_id.to_string().into(),
            xp_per_message: 15,
            xp_per_voice_minute: 5,
            xp_cooldown_secs: 60,
            level_up_channel_id: None,
            level_up_message: "Bravo {user} ! Tu as atteint le niveau {level} !".into(),
            excluded_channels: vec![],
            enabled: true,
            created_at: now,
            updated_at: now,
        })
    }

    async fn save_config(&self, cmd: SaveLevelConfigCommand) -> Result<LevelConfig, DomainError> {
        // Validation des bornes
        if cmd.xp_per_message < 1 || cmd.xp_per_message > 1000 {
            return Err(DomainError::ValidationError("xp_per_message doit etre entre 1 et 1000".into()));
        }
        if cmd.xp_per_voice_minute < 1 || cmd.xp_per_voice_minute > 1000 {
            return Err(DomainError::ValidationError("xp_per_voice_minute doit etre entre 1 et 1000".into()));
        }
        if cmd.xp_cooldown_secs < 0 || cmd.xp_cooldown_secs > 3600 {
            return Err(DomainError::ValidationError("xp_cooldown_secs doit etre entre 0 et 3600".into()));
        }

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
        // Validation
        if cmd.amount <= 0 {
            return Err(DomainError::ValidationError("Le montant XP doit etre positif".into()));
        }
        if cmd.amount > 10000 {
            return Err(DomainError::ValidationError("Le montant XP ne peut pas depasser 10000".into()));
        }

        // UPDATE atomique. RETURNING retourne les levels PRE-update (le SQL
        // ne modifie pas les colonnes level_*), ce qui elimine la race condition
        // entre lecture de l'ancien etat et l'update.
        let user_level_pre = self.repo.add_xp_atomic(
            &cmd.guild_id,
            &cmd.user_id,
            &cmd.username,
            cmd.amount,
            cmd.source,
        ).await?;

        // Anciens niveaux = ceux retournes par RETURNING (non touches par l'UPDATE).
        let old_level_text = user_level_pre.level_text;
        let old_level_voice = user_level_pre.level_voice;

        // Recalculer les niveaux depuis le nouvel XP.
        let mut user_level = user_level_pre;
        user_level.level = level_from_xp(user_level.xp);
        user_level.level_text = level_from_xp(user_level.xp_text);
        user_level.level_voice = level_from_xp(user_level.xp_voice);

        // Persister les niveaux recalcules.
        if let Err(e) = self.repo.upsert_user_level(&user_level).await {
            tracing::error!(
                error = %e,
                guild_id = %cmd.guild_id,
                user_id = %cmd.user_id,
                xp = user_level.xp,
                level = user_level.level,
                "Echec mise a jour niveaux apres ajout XP"
            );
            return Err(e);
        }

        // Detecter le level-up de la source specifique
        let (old_source_level, new_source_level) = match cmd.source {
            XpSource::Text => (old_level_text, user_level.level_text),
            XpSource::Voice => (old_level_voice, user_level.level_voice),
            XpSource::Days => (0, 0),
        };

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
            reward_role_id: reward_role_id.map(Into::into),
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
            id: uuid::Uuid::new_v4(),
            guild_id: guild_id.to_string().into(),
            level,
            role_id: role_id.into(),
            source,
        };
        self.repo.upsert_reward(&reward).await?;
        Ok(reward)
    }

    async fn delete_reward(&self, guild_id: &str, level: i32, source: XpSource) -> Result<(), DomainError> {
        self.repo.delete_reward(guild_id, level, source).await
    }

    async fn set_user_xp(&self, cmd: SetUserXpCommand) -> Result<UserLevel, DomainError> {
        // Charge l'existant (404 si pas trouve : on force a passer par add_xp avant pour creer la ligne)
        let mut user = self
            .repo
            .get_user_level(cmd.guild_id.as_ref(), cmd.user_id.as_ref())
            .await?
            .ok_or_else(|| DomainError::NotFound(format!(
                "User {} n'a pas encore de progression sur la guild {}",
                cmd.user_id.as_ref(), cmd.guild_id.as_ref()
            )))?;

        if let Some(xp_t) = cmd.xp_text {
            if xp_t < 0 {
                return Err(DomainError::ValidationError("xp_text doit etre >= 0".into()));
            }
            user.xp_text = xp_t;
            user.level_text = level_from_xp(xp_t);
        }
        if let Some(xp_v) = cmd.xp_voice {
            if xp_v < 0 {
                return Err(DomainError::ValidationError("xp_voice doit etre >= 0".into()));
            }
            user.xp_voice = xp_v;
            user.level_voice = level_from_xp(xp_v);
        }
        // Recalcule le total a partir des deux sources.
        user.xp = user.xp_text + user.xp_voice;
        user.level = level_from_xp(user.xp);
        user.updated_at = Utc::now();

        self.repo.upsert_user_level(&user).await?;
        // Force la MV a jour pour que le leaderboard "global" voie la
        // nouvelle valeur immediatement (sinon staleness ~5min).
        let _ = self.repo.refresh_leaderboard_view().await;
        Ok(user)
    }

    async fn reset_user_xp(&self, guild_id: &str, user_id: &str, target: ResetTarget) -> Result<UserLevel, DomainError> {
        let mut user = self
            .repo
            .get_user_level(guild_id, user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!(
                "User {user_id} n'a pas encore de progression sur la guild {guild_id}"
            )))?;

        match target {
            ResetTarget::Text => {
                user.xp_text = 0;
                user.level_text = 0;
            }
            ResetTarget::Voice => {
                user.xp_voice = 0;
                user.level_voice = 0;
            }
            ResetTarget::All => {
                user.xp_text = 0;
                user.level_text = 0;
                user.xp_voice = 0;
                user.level_voice = 0;
            }
        }
        user.xp = user.xp_text + user.xp_voice;
        user.level = level_from_xp(user.xp);
        user.updated_at = Utc::now();

        self.repo.upsert_user_level(&user).await?;
        let _ = self.repo.refresh_leaderboard_view().await;
        Ok(user)
    }
}

#[cfg(test)]
#[path = "tests/manage_levels.rs"]
mod tests;
