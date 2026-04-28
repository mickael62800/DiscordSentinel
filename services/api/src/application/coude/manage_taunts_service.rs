//! Impl du use case taunts (Phase 9 Part D + migration 139).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;
use tracing::warn;
use crate::domain::entities::coude::taunt::build_taunt_event;
use crate::domain::entities::coude::taunt::build_taunt_event_single;
use crate::domain::entities::coude::taunt::crossed_threshold;
use crate::domain::entities::system::config_parsers::parse_bool_config;
use crate::domain::entities::system::config_parsers::parse_i64_config;
use crate::domain::entities::coude::taunt::CoudeTauntsConfig;
use crate::domain::entities::coude::curse::CurseKind;
use crate::domain::entities::coude::taunt::StreakKind;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::coude::curses_repository::CoudeCursesRepository;
use crate::ports::outbound::coude::player_repository::CoudePlayerRepository;
use crate::ports::outbound::coude::taunts_repository::CoudeTauntsRepository;
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
    curses_repo: Option<Arc<dyn CoudeCursesRepository>>,
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
            curses_repo: None,
        }
    }

    /// Branche le repo des maledictions pour activer Insomnia
    /// (cf. COUPE_AMELIORATIONS 5.1) : la cible voit ses paliers de
    /// taunts de defaite atteints +50% plus vite (streak effectif x1.5).
    pub fn with_curses_repo(mut self, repo: Arc<dyn CoudeCursesRepository>) -> Self {
        self.curses_repo = Some(repo);
        self
    }

    /// Retourne true si le joueur est sous l effet Insomnia.
    async fn has_insomnia(&self, guild_id: &str, user_id: &str) -> bool {
        let Some(repo) = &self.curses_repo else {
            return false;
        };
        matches!(
            repo.get_active_for_target(guild_id, user_id).await,
            Ok(Some(c)) if c.kind == CurseKind::Insomnia
        )
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

        // Insomnia (cf. COUPE_AMELIORATIONS 5.1) — multiplie les streaks
        // de defaite par 1.5 avant de checker les paliers, pour que les
        // taunts tombent +50% plus vite sur la cible maudite.
        let effective_streak = if matches!(kind, StreakKind::Loss)
            && self.has_insomnia(guild_id, user_id).await
        {
            ((new_streak as f64) * 1.5).floor() as i32
        } else {
            new_streak
        };

        if crossed_threshold(effective_streak).is_none() {
            return Ok(None);
        }
        let new_streak = effective_streak;

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

// parse_bool_config / parse_i64_config vivent dans domain/entities/config_parsers.rs
// (purs et reutilisables par d'autres services qui lisent bot_guild_config).

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
        if !parse_bool_config(&cfg, CFG_BANKRUPTCY_ENABLED, true) {
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
        let threshold = parse_i64_config(&cfg, CFG_JACKPOT_THRESHOLD, DEFAULT_JACKPOT_THRESHOLD);
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
        let threshold = parse_i64_config(&cfg, CFG_DONOR_THRESHOLD, DEFAULT_DONOR_THRESHOLD);
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

#[cfg(test)]
#[path = "tests/manage_taunts.rs"]
mod tests;
