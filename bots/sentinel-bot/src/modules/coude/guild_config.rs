#![allow(dead_code)]
//! Lecture de la configuration per-guild depuis l'API (table bot_guild_config).
//! Fournit des valeurs par defaut si non configurees.

use std::collections::HashMap;
use sentinel_shared::api_client::BaseApiClient;

/// Mode d'agregation des deux d20 de Double Coup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleCoupMode {
    Max,
    Median,
    Min,
}

impl DoubleCoupMode {
    /// Parse depuis la config. Fallback `Median` si valeur inconnue.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "max" => Self::Max,
            "min" => Self::Min,
            "median" | "mediane" | "" => Self::Median,
            _ => Self::Median,
        }
    }

    /// Applique la strategie d'agregation sur deux rolls.
    pub fn aggregate(self, a: i32, b: i32) -> i32 {
        match self {
            Self::Max => a.max(b),
            Self::Min => a.min(b),
            // Mediane de deux valeurs = moyenne arrondie au plus proche.
            Self::Median => (a + b) / 2,
        }
    }
}

/// Configuration du jeu Coup de Coude pour une guild.
/// Toutes les valeurs ont un defaut raisonnable.
pub struct CoudeConfig {
    raw: HashMap<String, String>,
}

impl CoudeConfig {
    /// Charge la config guild depuis l'API.
    pub async fn load(api: &BaseApiClient, guild_id: &str) -> Self {
        let raw = match api.get_guild_config_for(guild_id, crate::modules::coude::MODULE_BOT_NAME).await {
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

    /// Nombre max de vols par jour (defaut: 5, 0 = illimite).
    pub fn steal_max_daily(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "steal_max_daily", 5)
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
            // Braquage (Phase 10)
            "masque_braquage" => 100,
            "pied_de_biche" => 150,
            "crochet_vault" => 220,
            "plan_coffre" => 320,
            "fumigene_diversion" => 450,
            "explosif" => 600,
            "hacker_kit" => 800,
            "drone_espion" => 1000,
            "equipe_de_pros" => 1500,
            _ => 100,
        };
        BaseApiClient::config_u64(&self.raw, &config_key, default) as i64
    }

    // ── Classe ──

    pub fn class_change_cost(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "class_change_cost", 500) as i64
    }

    // ── Don ──

    pub fn gift_min_coins(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "gift_min_coins", 10) as i64
    }

    pub fn gift_min_coins_after(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "gift_min_coins_after", 50) as i64
    }

    pub fn gift_tax_rate(&self) -> f64 {
        BaseApiClient::config_u64(&self.raw, "gift_tax_percent", 10) as f64 / 100.0
    }

    pub fn gift_cooldown_secs(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "gift_cooldown_secs", 3600) as i64
    }

    // ── Reset stats ──

    pub fn reset_stats_cost(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "reset_stats_cost", 300) as i64
    }

    // ── Repos ──

    pub fn repos_cooldown_hours(&self) -> i64 {
        BaseApiClient::config_u64(&self.raw, "repos_cooldown_hours", 12) as i64
    }

    // ── HP regen (affichage seulement — le calcul reel est dans le worker/API) ──

    pub fn hp_regen_rate_0_25(&self) -> f64 {
        BaseApiClient::config_u64(&self.raw, "hp_regen_rate_0_25", 100) as f64
    }

    pub fn hp_regen_rate_25_50(&self) -> f64 {
        BaseApiClient::config_u64(&self.raw, "hp_regen_rate_25_50", 50) as f64
    }

    pub fn hp_regen_rate_50_75(&self) -> f64 {
        BaseApiClient::config_u64(&self.raw, "hp_regen_rate_50_75", 30) as f64
    }

    pub fn hp_regen_rate_75_100(&self) -> f64 {
        BaseApiClient::config_u64(&self.raw, "hp_regen_rate_75_100", 10) as f64
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

    // ── Balance (Phase 132 : parametres rendus configurables) ──
    //
    // Note : la plupart de ces valeurs sont consommees par l'API (moteur
    // de combat, vol, braquage). Le bot expose les getters pour que l'UI
    // Desktop puisse editer la config et pour les rares cas ou le bot
    // applique lui-meme la regle (ex: steal_failure_penalty_pct dans
    // voler.rs). Les autres getters sont disponibles pour futur wiring.

    /// PV min (%) attaquant requis pour utiliser Surprise. 0 = desactive.
    pub fn surprise_min_hp_percent(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "surprise_min_hp_percent", 40)
    }

    /// Defenseur peut-il repondre avec Explosion a une Surprise ?
    pub fn surprise_allow_defender_counter(&self) -> bool {
        BaseApiClient::config_bool(&self.raw, "surprise_allow_defender_counter", true)
    }

    /// Max boosts voleur simultanes actifs (0 = illimite).
    pub fn steal_max_active_boosts(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "steal_max_active_boosts", 3)
    }

    /// % coins perdus par le voleur si son vol echoue.
    pub fn steal_failure_penalty_pct(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "steal_failure_penalty_pct", 20)
    }

    /// % outils consommes si braquage reussit.
    pub fn braquage_tools_consumed_success_pct(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "braquage_tools_consumed_success_pct", 50)
    }

    /// % outils consommes si braquage echoue.
    pub fn braquage_tools_consumed_fail_pct(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "braquage_tools_consumed_fail_pct", 25)
    }

    /// Mode d'agregation des 2d20 de Double Coup : max | median | min.
    pub fn double_coup_mode(&self) -> DoubleCoupMode {
        let raw = BaseApiClient::config_or(&self.raw, "double_coup_mode", "median");
        DoubleCoupMode::parse(&raw)
    }

    /// +% ATK applique par Rage.
    pub fn rage_atk_bonus_pct(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "rage_atk_bonus_pct", 40)
    }

    /// -% DEF applique par Rage.
    pub fn rage_def_malus_pct(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "rage_def_malus_pct", 15)
    }

    /// -% DEF applique au defenseur par Coup Traitre.
    pub fn coup_traitre_def_malus_pct(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "coup_traitre_def_malus_pct", 40)
    }

    /// +% DEF applique par Bouclier.
    pub fn bouclier_def_bonus_pct(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "bouclier_def_bonus_pct", 20)
    }

    /// PV perdus par round avec Poison.
    pub fn poison_damage_per_round(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "poison_damage_per_round", 5)
    }

    pub fn channel_combats(&self) -> Option<String> { self.channel_opt("channel_combats") }
    pub fn channel_leaderboard(&self) -> Option<String> { self.channel_opt("channel_leaderboard") }
    pub fn channel_profil(&self) -> Option<String> { self.channel_opt("channel_profil") }
    pub fn channel_activites(&self) -> Option<String> { self.channel_opt("channel_activites") }
    pub fn channel_announcements(&self) -> Option<String> { self.channel_opt("channel_announcements") }
    pub fn channel_notifications(&self) -> Option<String> { self.channel_opt("channel_notifications") }
}
