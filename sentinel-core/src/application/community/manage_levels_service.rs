use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::entities::community::level::level_from_xp;
use crate::domain::entities::community::level::UserLevel;
use crate::domain::entities::community::level::XpSource;
use crate::domain::errors::DomainError;
use crate::ports::inbound::community::manage_levels::AddXpCommand;
use crate::ports::inbound::community::manage_levels::AddXpResult;
use crate::ports::inbound::community::manage_levels::ManageLevelsUseCase;
use crate::ports::inbound::community::manage_levels::ResetTarget;
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
    async fn add_xp(&self, cmd: AddXpCommand) -> Result<AddXpResult, DomainError> {
        // Validation
        crate::application::validation::validate_positive(cmd.amount, "Le montant XP")?;
        if cmd.amount > 10000 {
            return Err(DomainError::ValidationError(
                "Le montant XP ne peut pas depasser 10000".into(),
            ));
        }

        // UPDATE atomique. RETURNING retourne les levels PRE-update (le SQL
        // ne modifie pas les colonnes level_*), ce qui elimine la race condition
        // entre lecture de l'ancien etat et l'update.
        let user_level_pre = self
            .repo
            .add_xp_atomic(
                &cmd.guild_id,
                &cmd.user_id,
                &cmd.username,
                cmd.amount,
                cmd.source,
            )
            .await?;

        // Anciens niveaux = ceux retournes par RETURNING (non touches par l'UPDATE).
        let old_level_text = user_level_pre.level_text;
        let old_level_voice = user_level_pre.level_voice;
        let old_level_global = user_level_pre.level;

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
        };

        let leveled_up = new_source_level > old_source_level;

        Ok(AddXpResult {
            user_level,
            leveled_up,
            old_level: old_source_level,
            old_level_global,
            source: cmd.source,
        })
    }

    async fn get_user_level(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UserLevel, DomainError> {
        self.repo
            .get_user_level(guild_id, user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Niveau introuvable pour {user_id}")))
    }

    async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError> {
        self.repo.get_leaderboard(guild_id, limit).await
    }

    async fn get_leaderboard_by_source(
        &self,
        guild_id: &str,
        source: XpSource,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError> {
        self.repo
            .get_leaderboard_by_source(guild_id, source, limit)
            .await
    }

    async fn set_user_xp(&self, cmd: SetUserXpCommand) -> Result<UserLevel, DomainError> {
        let mut user = self
            .repo
            .get_user_level(cmd.guild_id.as_ref(), cmd.user_id.as_ref())
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!(
                    "User {} n'a pas encore de progression sur la guild {}",
                    cmd.user_id.as_ref(),
                    cmd.guild_id.as_ref()
                ))
            })?;

        if let Some(xp_t) = cmd.xp_text {
            if xp_t < 0 {
                return Err(DomainError::ValidationError(
                    "xp_text doit etre >= 0".into(),
                ));
            }
            user.xp_text = xp_t;
            user.level_text = level_from_xp(xp_t);
        }
        if let Some(xp_v) = cmd.xp_voice {
            if xp_v < 0 {
                return Err(DomainError::ValidationError(
                    "xp_voice doit etre >= 0".into(),
                ));
            }
            user.xp_voice = xp_v;
            user.level_voice = level_from_xp(xp_v);
        }
        user.xp = user.xp_text + user.xp_voice;
        user.level = level_from_xp(user.xp);
        user.updated_at = Utc::now();

        self.repo.upsert_user_level(&user).await?;
        let _ = self.repo.refresh_leaderboard_view().await;
        Ok(user)
    }

    async fn reset_user_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        target: ResetTarget,
    ) -> Result<UserLevel, DomainError> {
        let mut user = self
            .repo
            .get_user_level(guild_id, user_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!(
                    "User {user_id} n'a pas encore de progression sur la guild {guild_id}"
                ))
            })?;

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
