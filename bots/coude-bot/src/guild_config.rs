//! Lecture de la configuration per-guild depuis l'API (table bot_guild_config).
//! Fournit des valeurs par defaut si non configurees.

use std::collections::HashMap;
use sentinel_shared::api_client::BaseApiClient;

/// Configuration du jeu Coup de Coude pour une guild.
/// Toutes les valeurs ont un defaut raisonnable.
pub struct CoudeConfig {
    raw: HashMap<String, String>,
}

impl CoudeConfig {
    /// Charge la config guild depuis l'API.
    pub async fn load(api: &BaseApiClient, guild_id: &str) -> Self {
        let raw = match api.get_guild_config(guild_id).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %guild_id, "Echec get_guild_config");
                std::collections::HashMap::new()
            }
        };
        Self { raw }
    }

    /// Bot active pour cette guild ?
    pub fn enabled(&self) -> bool {
        BaseApiClient::config_bool(&self.raw, "enabled", true)
    }

    // ── Coins ──

    pub fn starting_coins(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "starting_coins", 200) as i64
    }

    pub fn min_bet(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "min_bet", 1) as i64
    }

    pub fn max_bet(&self) -> i64 {
        let v = BaseApiClient::config_u64(&self.raw, "max_bet", 0) as i64;
        if v == 0 { i64::MAX } else { v }
    }

    pub fn default_bet(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "default_bet", 10) as i64
    }

    // ── Chaos ──

    pub fn chaos_enabled(&self) -> bool {
        BaseApiClient::config_bool(&self.raw, "chaos_enabled", true)
    }

    pub fn chaos_chance(&self) -> u32 {
        BaseApiClient::config_u64(&self.raw, "chaos_chance", 18) as u32
    }

    // ── Casino ──

    pub fn casino_enabled(&self) -> bool {
        BaseApiClient::config_bool(&self.raw, "casino_enabled", true)
    }

    pub fn casino_max_bet(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "casino_max_bet", 500) as i64
    }

    /// Duree d'expiration d'un defi en secondes (defaut: 86400 = 24h).
    pub fn combat_expire_secs(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "combat_expire_secs", 86400)
    }

    /// Cooldown entre chaque /casino en secondes (defaut: 300 = 5 min).
    pub fn casino_cooldown_secs(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "casino_cooldown_secs", 300) as i64
    }

    /// Nombre max de /casino par jour (defaut: 10, 0 = illimite).
    pub fn casino_max_daily(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "casino_max_daily", 10)
    }

    /// Gain max par jour au casino (defaut: 5000, 0 = illimite).
    pub fn casino_max_daily_gain(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "casino_max_daily_gain", 5000) as i64
    }

    // ── Vol ──

    pub fn steal_enabled(&self) -> bool {
        BaseApiClient::config_bool(&self.raw, "steal_enabled", true)
    }

    pub fn steal_success_rate(&self) -> u32 {
        BaseApiClient::config_u64(&self.raw, "steal_success_rate", 30) as u32
    }

    pub fn steal_cooldown_secs(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "steal_cooldown_secs", 1800) as i64
    }

    // ── Assurance ──

    pub fn insurance_cost(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "insurance_cost", 50) as i64
    }

    pub fn insurance_duration_secs(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "insurance_duration_secs", 3600) as i64
    }

    pub fn insurance_scam_rate(&self) -> u32 {
        BaseApiClient::config_u64(&self.raw, "insurance_scam_rate", 5) as u32
    }

    // ── Lachete ──

    pub fn cowardice_threshold(&self) -> i32 {
        BaseApiClient::config_u64(&self.raw, "cowardice_threshold", 5) as i32
    }

    pub fn cowardice_penalty(&self) -> f64 {
        BaseApiClient::config_u64(&self.raw, "cowardice_penalty", 20) as f64 / 100.0
    }

    pub fn refusal_penalty(&self) -> f64 {
        BaseApiClient::config_u64(&self.raw, "refusal_penalty", 20) as f64 / 100.0
    }

    // ── Annulation ──

    pub fn cancel_penalty(&self) -> f64 {
        BaseApiClient::config_u64(&self.raw, "cancel_penalty", 5) as f64 / 100.0
    }

    // ── Delai de paris ──

    pub fn bet_delay_secs(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "bet_delay_secs", 300)
    }

    // ── Daily Chaos ──

    pub fn daily_chaos_enabled(&self) -> bool {
        BaseApiClient::config_bool(&self.raw, "daily_chaos_enabled", true)
    }

    pub fn daily_chaos_percent(&self) -> f64 {
        BaseApiClient::config_u64(&self.raw, "daily_chaos_percent", 20) as f64 / 100.0
    }

    // ── Happy Hour ──

    pub fn happy_hour_multiplier(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "happy_hour_multiplier", 2) as i64
    }

    // ── Shop prices ──

    pub fn shop_price(&self, item_key: &str) -> i64 {
        let config_key = format!("shop_{}_price", item_key);
        let default = match item_key {
            "potion_soin" => 80,
            "rage" => 100,
            "mindgame" => 150,
            "antidote" => 150,
            "explosion" => 200,
            "potion_majeure" => 200,
            "double_coup" => 250,
            "bouclier" => 250,
            "surprise" => 300,
            "poison" => 300,
            "coup_traitre" => 350,
            _ => 100,
        };
        BaseApiClient::config_u64(&self.raw, &config_key, default) as i64
    }

    // ── Log channel ──

    pub fn log_channel_id(&self) -> Option<String> {
        let v = BaseApiClient::config_or(&self.raw, "log_channel_id", "");
        if v.is_empty() { None } else { Some(v) }
    }

    // ── Salons par groupe de commandes ──

    fn channel_opt(&self, key: &str) -> Option<String> {
        let v = BaseApiClient::config_or(&self.raw, key, "");
        if v.is_empty() { None } else { Some(v) }
    }

    pub fn channel_combats(&self) -> Option<String> { self.channel_opt("channel_combats") }
    pub fn channel_leaderboard(&self) -> Option<String> { self.channel_opt("channel_leaderboard") }
    pub fn channel_profil(&self) -> Option<String> { self.channel_opt("channel_profil") }
    pub fn channel_activites(&self) -> Option<String> { self.channel_opt("channel_activites") }
    pub fn channel_announcements(&self) -> Option<String> { self.channel_opt("channel_announcements") }
    pub fn channel_notifications(&self) -> Option<String> { self.channel_opt("channel_notifications") }
}
