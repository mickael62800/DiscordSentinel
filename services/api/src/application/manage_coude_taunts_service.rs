//! Impl du use case taunts (Phase 9 Part D).

use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use crate::domain::entities::{
    build_taunt_event, crossed_threshold, CoudeTauntsConfig, StreakKind, TauntEvent,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_taunts::ManageCoudeTauntsUseCase;
use crate::ports::outbound::{CoudePlayerRepository, CoudeTauntsRepository};

pub struct ManageCoudeTauntsService {
    taunts_repo: Arc<dyn CoudeTauntsRepository>,
    player_repo: Arc<dyn CoudePlayerRepository>,
}

impl ManageCoudeTauntsService {
    pub fn new(
        taunts_repo: Arc<dyn CoudeTauntsRepository>,
        player_repo: Arc<dyn CoudePlayerRepository>,
    ) -> Self {
        Self {
            taunts_repo,
            player_repo,
        }
    }

    /// Helper commun : met a jour la streak, charge la config, decide de
    /// produire un event. Factorise les 3 chemins win/loss/steal.
    async fn handle_streak_touch(
        &self,
        guild_id: &str,
        user_id: &str,
        kind: StreakKind,
        new_streak: Option<i32>,
    ) -> Result<Option<TauntEvent>, DomainError> {
        let Some(new_streak) = new_streak else {
            debug!(guild_id, user_id, kind = kind.as_str(), "taunt: joueur introuvable (streak None)");
            return Ok(None);
        };

        // Diagnostic : log la streak a chaque touch pour tracer la progression.
        debug!(guild_id, user_id, kind = kind.as_str(), new_streak, "taunt: streak updated");

        // Pas un seuil (3/5/10) → pas de taunt, pas besoin de charger la config.
        if crossed_threshold(new_streak).is_none() {
            return Ok(None);
        }

        let config = self.taunts_repo.get_or_init_config(guild_id).await?;
        if !config.enabled {
            debug!(guild_id, "taunt: config disabled pour cette guild");
            return Ok(None);
        }
        if config.channel_id.is_none() {
            debug!(guild_id, "taunt: pas de channel_id configure");
            return Ok(None);
        }
        let opted_out = self.taunts_repo.is_opted_out(guild_id, user_id).await?;
        if opted_out {
            debug!(guild_id, user_id, "taunt: joueur a opt-out");
            return Ok(None);
        }
        let event = build_taunt_event(&config, user_id, kind, new_streak, opted_out);
        if event.is_some() {
            debug!(guild_id, user_id, kind = kind.as_str(), new_streak, "taunt: event emis !");
        }
        Ok(event)
    }
}

#[async_trait]
impl ManageCoudeTauntsUseCase for ManageCoudeTauntsService {
    async fn on_player_won(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError> {
        let new_streak = self.player_repo.touch_win_streak(guild_id, user_id).await?;
        self.handle_streak_touch(guild_id, user_id, StreakKind::Win, new_streak)
            .await
    }

    async fn on_player_lost(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError> {
        let new_streak = self.player_repo.touch_loss_streak(guild_id, user_id).await?;
        self.handle_streak_touch(guild_id, user_id, StreakKind::Loss, new_streak)
            .await
    }

    async fn on_player_drew(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        self.player_repo.reset_combat_streaks(guild_id, user_id).await
    }

    async fn on_player_stolen_from(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError> {
        let new_streak = self
            .player_repo
            .touch_steal_victim_streak(guild_id, user_id)
            .await?;
        self.handle_streak_touch(guild_id, user_id, StreakKind::StealVictim, new_streak)
            .await
    }

    async fn on_player_defended_steal(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        self.player_repo.reset_steal_victim_streak(guild_id, user_id).await
    }

    async fn get_config(&self, guild_id: &str) -> Result<CoudeTauntsConfig, DomainError> {
        self.taunts_repo.get_or_init_config(guild_id).await
    }

    async fn set_channel(
        &self,
        guild_id: &str,
        channel_id: Option<&str>,
    ) -> Result<(), DomainError> {
        self.taunts_repo.set_channel(guild_id, channel_id).await
    }

    async fn set_enabled(&self, guild_id: &str, enabled: bool) -> Result<(), DomainError> {
        self.taunts_repo.set_enabled(guild_id, enabled).await
    }

    async fn set_opt_out(
        &self,
        guild_id: &str,
        user_id: &str,
        opted_out: bool,
    ) -> Result<(), DomainError> {
        self.taunts_repo
            .set_opt_out(guild_id, user_id, opted_out)
            .await
    }

    async fn is_opted_out(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError> {
        self.taunts_repo.is_opted_out(guild_id, user_id).await
    }

    async fn list_opt_outs(&self, guild_id: &str) -> Result<Vec<String>, DomainError> {
        self.taunts_repo.list_opt_outs(guild_id).await
    }
}
