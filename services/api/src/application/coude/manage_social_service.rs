#[cfg(test)]
#[path = "tests/manage_social.rs"]
mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use tracing::info;

use crate::domain::entities::coude::social::clamp_leaderboard_limit;
use crate::domain::entities::coude::social::daily_chaos_amount;
use crate::domain::entities::coude::social::Season;
use crate::domain::entities::coude::social::Event;
use crate::domain::entities::coude::social::LeaderboardEntry;
use crate::domain::entities::coude::social::DailyChaosOutcome;
use crate::domain::entities::coude::social::LeaderboardCategory;
use crate::domain::entities::coude::social::NewDailyChaos;
use crate::domain::entities::coude::social::DAILY_CHAOS_MAX;
use crate::domain::entities::coude::social::DEFAULT_CHAOS_PERCENT;
use crate::domain::entities::coude::social::MIN_COINS_ELIGIBLE;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_social::ManageCoudeSocialUseCase;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::coude::economy_repository::EconomyRepository;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::coude::social_repository::SocialRepository;
// DAILY_CHAOS_MAX / DEFAULT_CHAOS_PERCENT / MIN_COINS_ELIGIBLE vivent
// dans domain/entities/coude_social.rs (regles metier reutilisables).

pub struct ManageCoudeSocialService {
    repo: Arc<dyn SocialRepository>,
    player_repo: Arc<dyn PlayerRepository>,
    economy_repo: Arc<dyn EconomyRepository>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
    wallet_uc: Arc<dyn ManageWalletUseCase>,
}

impl ManageCoudeSocialService {
    pub fn new(
        repo: Arc<dyn SocialRepository>,
        player_repo: Arc<dyn PlayerRepository>,
        economy_repo: Arc<dyn EconomyRepository>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
        wallet_uc: Arc<dyn ManageWalletUseCase>,
    ) -> Self {
        Self { repo, player_repo, economy_repo, bot_config_repo, wallet_uc }
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
    ) -> Result<Vec<LeaderboardEntry>, DomainError> {
        let limit = clamp_leaderboard_limit(limit);
        self.repo.leaderboard(guild_id, category, limit).await
    }

    async fn list_active_events(&self, guild_id: &str) -> Result<Vec<Event>, DomainError> {
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

        // 4. Calculer le montant via regle domain (None si < 1 coin).
        let Some(amount) = daily_chaos_amount(victim.coins, chaos_percent) else {
            return Ok(None);
        };

        // 5. Migration #5 : transfert via ManageWalletUseCase (faillite
        //    victime + jackpot winner auto-detectes, log atomique dans
        //    wallet_transactions). Le `amount` est deja clamp par le
        //    calcul 20% d'un solde > MIN_COINS_ELIGIBLE.
        let description = format!(
            "Daily chaos ({} -> {})",
            victim.user_id, winner.user_id
        );
        let taunts = self
            .wallet_uc
            .transfer(
                guild_id,
                &victim.user_id,
                &winner.user_id,
                amount,
                "coude_daily_chaos",
                &description,
            )
            .await?;
        // Stats compteurs (total_lost victime / total_stolen+earned winner).
        self.economy_repo
            .record_steal_stats(guild_id, &winner.user_id, &victim.user_id, amount)
            .await?;
        let actual = amount;

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
            taunts = taunts.len(),
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
            taunt_events: taunts,
        }))
    }

    async fn current_season(&self, guild_id: &str) -> Result<Season, DomainError> {
        self.repo.get_or_bootstrap_current_season(guild_id).await
    }
}
