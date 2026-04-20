//! Impl du use case taunts (Phase 9 Part D + migration 139).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, warn};

use crate::domain::entities::{
    build_taunt_event, build_taunt_event_single, crossed_threshold, CoudeTauntsConfig, StreakKind,
    TauntEvent,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_taunts::ManageCoudeTauntsUseCase;
use crate::ports::outbound::{BotConfigRepository, CoudePlayerRepository, CoudeTauntsRepository};

const ECO_BOT_NAME: &str = "coude-bot";
const CFG_BANKRUPTCY_ENABLED: &str = "bankruptcy_taunt_enabled";
const CFG_JACKPOT_THRESHOLD: &str = "jackpot_threshold";
const CFG_DONOR_THRESHOLD: &str = "generous_donor_threshold";
const DEFAULT_JACKPOT_THRESHOLD: i64 = 10_000;
const DEFAULT_DONOR_THRESHOLD: i64 = 1_000;

pub struct ManageCoudeTauntsService {
    taunts_repo: Arc<dyn CoudeTauntsRepository>,
    player_repo: Arc<dyn CoudePlayerRepository>,
    bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl ManageCoudeTauntsService {
    pub fn new(
        taunts_repo: Arc<dyn CoudeTauntsRepository>,
        player_repo: Arc<dyn CoudePlayerRepository>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self {
            taunts_repo,
            player_repo,
            bot_config_repo,
        }
    }

    /// Helper commun threshold-based : met a jour la streak, charge la
    /// config, decide de produire un event.
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

        debug!(guild_id, user_id, kind = kind.as_str(), new_streak, "taunt: streak updated");

        if crossed_threshold(new_streak).is_none() {
            return Ok(None);
        }

        let (config, opted_out) = self.load_gate(guild_id, user_id).await?;
        let Some(config) = config else {
            return Ok(None);
        };
        let event = build_taunt_event(&config, user_id, kind, new_streak, opted_out);
        if event.is_some() {
            debug!(guild_id, user_id, kind = kind.as_str(), new_streak, "taunt: event emis !");
        }
        Ok(event)
    }

    /// Helper pour les one-shots : pas de streak, juste check config + opt-out.
    async fn handle_one_shot(
        &self,
        guild_id: &str,
        user_id: &str,
        kind: StreakKind,
    ) -> Result<Option<TauntEvent>, DomainError> {
        let (config, opted_out) = self.load_gate(guild_id, user_id).await?;
        let Some(config) = config else {
            return Ok(None);
        };
        Ok(build_taunt_event_single(&config, user_id, kind, opted_out))
    }

    /// Retourne la config si feature enabled + channel present, + flag opt-out.
    async fn load_gate(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(Option<CoudeTauntsConfig>, bool), DomainError> {
        let config = self.taunts_repo.get_or_init_config(guild_id).await?;
        if !config.enabled || config.channel_id.is_none() {
            return Ok((None, false));
        }
        let opted_out = self.taunts_repo.is_opted_out(guild_id, user_id).await?;
        Ok((Some(config), opted_out))
    }

    async fn load_eco_config(&self, guild_id: &str) -> HashMap<String, String> {
        match self.bot_config_repo.get_config(guild_id, ECO_BOT_NAME).await {
            Ok(entries) => entries
                .into_iter()
                .map(|e| (e.config_key, e.config_value))
                .collect(),
            Err(e) => {
                warn!(error = %e, guild_id, "taunts: echec lecture bot_guild_config — defaults");
                HashMap::new()
            }
        }
    }
}

fn parse_bool(map: &HashMap<String, String>, key: &str, default: bool) -> bool {
    map.get(key)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(default)
}

fn parse_i64(map: &HashMap<String, String>, key: &str, default: i64) -> i64 {
    map.get(key).and_then(|v| v.parse::<i64>().ok()).unwrap_or(default)
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

    // ── Blackjack (migration 139) ──

    async fn on_bj_natural(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError> {
        // Naturel = egalement une victoire → utilise touch_bj_win_streak
        // pour incrementer la win streak et reset la bust streak. Le
        // taunt Natural21 est one-shot, independant du palier.
        let _ = self.player_repo.touch_bj_win_streak(guild_id, user_id).await?;
        self.handle_one_shot(guild_id, user_id, StreakKind::BjNatural21)
            .await
    }

    async fn on_bj_hand_won(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError> {
        let new_streak = self.player_repo.touch_bj_win_streak(guild_id, user_id).await?;
        self.handle_streak_touch(guild_id, user_id, StreakKind::BjWinStreak, new_streak)
            .await
    }

    async fn on_bj_hand_bust(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError> {
        let new_streak = self.player_repo.touch_bj_bust_streak(guild_id, user_id).await?;
        self.handle_streak_touch(guild_id, user_id, StreakKind::BjBustStreak, new_streak)
            .await
    }

    // ── Economie (migration 139) ──

    async fn on_bankruptcy(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError> {
        let cfg = self.load_eco_config(guild_id).await;
        if !parse_bool(&cfg, CFG_BANKRUPTCY_ENABLED, true) {
            debug!(guild_id, user_id, "taunt: bankruptcy desactive par config");
            return Ok(None);
        }
        self.handle_one_shot(guild_id, user_id, StreakKind::EcoBankruptcy)
            .await
    }

    async fn on_jackpot(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<Option<TauntEvent>, DomainError> {
        let cfg = self.load_eco_config(guild_id).await;
        let threshold = parse_i64(&cfg, CFG_JACKPOT_THRESHOLD, DEFAULT_JACKPOT_THRESHOLD);
        if amount < threshold {
            return Ok(None);
        }
        self.handle_one_shot(guild_id, user_id, StreakKind::EcoJackpot)
            .await
    }

    async fn on_generous_donor(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<Option<TauntEvent>, DomainError> {
        let cfg = self.load_eco_config(guild_id).await;
        let threshold = parse_i64(&cfg, CFG_DONOR_THRESHOLD, DEFAULT_DONOR_THRESHOLD);
        if amount < threshold {
            return Ok(None);
        }
        self.handle_one_shot(guild_id, user_id, StreakKind::EcoGenerousDonor)
            .await
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

    async fn set_rename_enabled(&self, guild_id: &str, rename_enabled: bool) -> Result<(), DomainError> {
        self.taunts_repo.set_rename_enabled(guild_id, rename_enabled).await
    }

    async fn set_messages_enabled(&self, guild_id: &str, messages_enabled: bool) -> Result<(), DomainError> {
        self.taunts_repo.set_messages_enabled(guild_id, messages_enabled).await
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
