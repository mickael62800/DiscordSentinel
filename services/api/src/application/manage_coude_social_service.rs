use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::info;

use crate::domain::entities::{
    CoudeCurrentSeason, CoudeEvent, CoudeLeaderboardEntry, DailyChaosOutcome,
    LeaderboardCategory, NewDailyChaos,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_social::ManageCoudeSocialUseCase;
use crate::ports::outbound::{
    BotConfigRepository, CoudeEconomyRepository, CoudePlayerRepository, CoudeSocialRepository,
};

/// Cap journalier du nombre d'events chaos par guild.
const DAILY_CHAOS_MAX: i64 = 5;
/// Pourcentage des coins de la victime transferes par defaut.
const DEFAULT_CHAOS_PERCENT: f64 = 0.20;
/// Minimum de coins requis pour etre eligible (evite les chaos a 0).
const MIN_COINS_ELIGIBLE: i64 = 10;

pub struct ManageCoudeSocialService {
    repo: Arc<dyn CoudeSocialRepository>,
    player_repo: Arc<dyn CoudePlayerRepository>,
    economy_repo: Arc<dyn CoudeEconomyRepository>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl ManageCoudeSocialService {
    pub fn new(
        repo: Arc<dyn CoudeSocialRepository>,
        player_repo: Arc<dyn CoudePlayerRepository>,
        economy_repo: Arc<dyn CoudeEconomyRepository>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self { repo, player_repo, economy_repo, bot_config_repo }
    }
}

#[async_trait]
impl ManageCoudeSocialUseCase for ManageCoudeSocialService {
    async fn check_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        self.repo.get_cooldown(guild_id, user_id, action).await
    }

    async fn set_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
        duration_secs: i64,
    ) -> Result<(), DomainError> {
        if duration_secs <= 0 {
            return Err(DomainError::ValidationError(
                "La duree doit etre positive".into(),
            ));
        }
        self.repo
            .set_cooldown(guild_id, user_id, action, duration_secs)
            .await
    }

    async fn leaderboard(
        &self,
        guild_id: &str,
        category: LeaderboardCategory,
        limit: i64,
    ) -> Result<Vec<CoudeLeaderboardEntry>, DomainError> {
        let limit = limit.clamp(1, 100);
        self.repo.leaderboard(guild_id, category, limit).await
    }

    async fn list_active_events(&self, guild_id: &str) -> Result<Vec<CoudeEvent>, DomainError> {
        self.repo.list_active_events(guild_id).await
    }

    async fn log_daily_chaos(&self, chaos: NewDailyChaos) -> Result<(), DomainError> {
        self.repo.log_daily_chaos(chaos).await
    }

    async fn trigger_daily_chaos(
        &self,
        guild_id: &str,
    ) -> Result<Option<DailyChaosOutcome>, DomainError> {
        // 1. Cap journalier.
        let today_count = self.repo.count_daily_chaos_today(guild_id).await?;
        if today_count >= DAILY_CHAOS_MAX {
            return Ok(None);
        }

        // 2. Lire le % depuis la config guild (default 20%).
        let configs = self.bot_config_repo.get_config(guild_id, "coude-bot").await?;
        let chaos_percent = configs
            .iter()
            .find(|c| c.config_key == "daily_chaos_percent")
            .and_then(|c| c.config_value.parse::<f64>().ok())
            .map(|v| v / 100.0)
            .unwrap_or(DEFAULT_CHAOS_PERCENT);

        // Lire le channel d'annonce.
        let channel_id = match configs
            .iter()
            .find(|c| c.config_key == "channel_announcements")
            .map(|c| c.config_value.clone())
        {
            Some(ch) if !ch.is_empty() => ch,
            _ => return Ok(None), // Pas de channel configure → skip.
        };

        // 3. Tirer 2 joueurs aleatoires avec assez de coins.
        let players = self
            .player_repo
            .random_active(guild_id, 2, MIN_COINS_ELIGIBLE)
            .await?;
        if players.len() < 2 {
            return Ok(None); // Pas assez de joueurs eligibles.
        }
        let victim = &players[0];
        let winner = &players[1];

        // 4. Calculer le montant.
        let amount = ((victim.coins as f64) * chaos_percent).floor() as i64;
        if amount < 1 {
            return Ok(None);
        }

        // 5. Transfert via economy repo (steal atomique).
        let actual = self
            .economy_repo
            .steal(guild_id, &winner.user_id, &victim.user_id, amount)
            .await?;
        if actual <= 0 {
            return Ok(None);
        }

        // 6. Log en DB.
        self.repo
            .log_daily_chaos(NewDailyChaos {
                guild_id: guild_id.to_string(),
                loser_id: victim.user_id.clone(),
                loser_name: victim.username.clone(),
                winner_id: winner.user_id.clone(),
                winner_name: winner.username.clone(),
                amount: actual,
            })
            .await?;

        info!(
            guild_id,
            loser = %victim.user_id,
            winner = %winner.user_id,
            actual,
            today = today_count + 1,
            "Daily chaos triggered"
        );

        Ok(Some(DailyChaosOutcome {
            loser_id: victim.user_id.clone(),
            loser_name: victim.username.clone(),
            winner_id: winner.user_id.clone(),
            winner_name: winner.username.clone(),
            amount: actual,
            channel_id,
        }))
    }

    async fn current_season(&self, guild_id: &str) -> Result<CoudeCurrentSeason, DomainError> {
        self.repo.get_or_bootstrap_current_season(guild_id).await
    }
}
