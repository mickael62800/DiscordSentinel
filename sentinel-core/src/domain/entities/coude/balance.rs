//! Parametres de balance du jeu Coup de Coude (Phase 132).
//!
//! Ces valeurs sont configurables par guild via `bot_guild_config`
//! (`bot_name = 'coude-bot'`) et sont passees en argument au moteur de
//! combat et au service de braquage. Defaults correspondent aux valeurs
//! historiquement hardcodees cote bot / domain.

use std::collections::HashMap;

/// Mode d'agregation des deux d20 de Double Coup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleCoupMode {
    Max,
    Median,
    Min,
}

impl DoubleCoupMode {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "max" => Self::Max,
            "min" => Self::Min,
            "median" | "mediane" | "" => Self::Median,
            _ => Self::Median,
        }
    }

    /// Agrege 2 rolls en fonction du mode.
    pub fn aggregate(self, a: i32, b: i32) -> i32 {
        match self {
            Self::Max => a.max(b),
            Self::Min => a.min(b),
            Self::Median => (a + b) / 2,
        }
    }
}

/// Parametres mecaniques du jeu Coup de Coude, charges depuis
/// `bot_guild_config` pour une guild donnee.
#[derive(Debug, Clone, Copy)]
pub struct BalanceParams {
    /// PV min (%) attaquant requis pour utiliser Surprise. 0 = desactive.
    pub surprise_min_hp_pct: u64,
    /// Defenseur peut repondre avec Explosion a une Surprise.
    pub surprise_allow_defender_counter: bool,
    /// Max boosts voleur simultanes (0 = illimite). Enforcer cote bot.
    pub steal_max_active_boosts: u64,
    /// % outils consommes si braquage reussit.
    pub braquage_tools_consumed_success_pct: u64,
    /// % outils consommes si braquage echoue.
    pub braquage_tools_consumed_fail_pct: u64,
    /// Mode d'agregation des 2d20 de Double Coup.
    pub double_coup_mode: DoubleCoupMode,
    /// +% ATK applique par Rage.
    pub rage_atk_bonus_pct: u64,
    /// -% DEF applique par Rage.
    pub rage_def_malus_pct: u64,
    /// -% DEF applique au defenseur par Coup Traitre.
    pub coup_traitre_def_malus_pct: u64,
    /// +% DEF applique par Bouclier.
    pub bouclier_def_bonus_pct: u64,
    /// PV perdus par round avec Poison.
    pub poison_damage_per_round: u64,
    /// PV min (%) requis pour les DEUX combattants pour engager un combat.
    /// 0 = desactive. Empeche un joueur a 0 HP de se faire defier.
    pub combat_min_hp_pct: u64,
    /// Delai entre 2 tentatives de braquage par joueur (en jours).
    pub heist_cooldown_days: u64,
    /// Duree de prison apres un braquage rate (en heures).
    pub heist_prison_hours: u64,
}

impl Default for BalanceParams {
    fn default() -> Self {
        Self {
            surprise_min_hp_pct: 40,
            surprise_allow_defender_counter: true,
            steal_max_active_boosts: 3,
            braquage_tools_consumed_success_pct: 50,
            braquage_tools_consumed_fail_pct: 25,
            double_coup_mode: DoubleCoupMode::Median,
            rage_atk_bonus_pct: 40,
            rage_def_malus_pct: 15,
            coup_traitre_def_malus_pct: 40,
            bouclier_def_bonus_pct: 20,
            poison_damage_per_round: 5,
            combat_min_hp_pct: 40,
            heist_cooldown_days: 7,
            heist_prison_hours: 24,
        }
    }
}

impl BalanceParams {
    /// Construit les parametres depuis une map cle→valeur (typiquement
    /// la config guild chargee depuis `bot_guild_config`). Toute cle
    /// manquante ou invalide retombe sur le default.
    pub fn from_config(cfg: &HashMap<String, String>) -> Self {
        let d = Self::default();
        Self {
            surprise_min_hp_pct: parse_u64(cfg, "surprise_min_hp_percent", d.surprise_min_hp_pct),
            surprise_allow_defender_counter: parse_bool(
                cfg,
                "surprise_allow_defender_counter",
                d.surprise_allow_defender_counter,
            ),
            steal_max_active_boosts: parse_u64(
                cfg,
                "steal_max_active_boosts",
                d.steal_max_active_boosts,
            ),
            braquage_tools_consumed_success_pct: parse_u64(
                cfg,
                "braquage_tools_consumed_success_pct",
                d.braquage_tools_consumed_success_pct,
            ),
            braquage_tools_consumed_fail_pct: parse_u64(
                cfg,
                "braquage_tools_consumed_fail_pct",
                d.braquage_tools_consumed_fail_pct,
            ),
            double_coup_mode: cfg
                .get("double_coup_mode")
                .map(|v| DoubleCoupMode::parse(v))
                .unwrap_or(d.double_coup_mode),
            rage_atk_bonus_pct: parse_u64(cfg, "rage_atk_bonus_pct", d.rage_atk_bonus_pct),
            rage_def_malus_pct: parse_u64(cfg, "rage_def_malus_pct", d.rage_def_malus_pct),
            coup_traitre_def_malus_pct: parse_u64(
                cfg,
                "coup_traitre_def_malus_pct",
                d.coup_traitre_def_malus_pct,
            ),
            bouclier_def_bonus_pct: parse_u64(
                cfg,
                "bouclier_def_bonus_pct",
                d.bouclier_def_bonus_pct,
            ),
            poison_damage_per_round: parse_u64(
                cfg,
                "poison_damage_per_round",
                d.poison_damage_per_round,
            ),
            combat_min_hp_pct: parse_u64(cfg, "combat_min_hp_pct", d.combat_min_hp_pct),
            heist_cooldown_days: parse_u64(cfg, "heist_cooldown_days", d.heist_cooldown_days),
            heist_prison_hours: parse_u64(cfg, "heist_prison_hours", d.heist_prison_hours),
        }
    }
}

fn parse_u64(cfg: &HashMap<String, String>, key: &str, default: u64) -> u64 {
    cfg.get(key)
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn parse_bool(cfg: &HashMap<String, String>, key: &str, default: bool) -> bool {
    cfg.get(key)
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

#[cfg(test)]
#[path = "tests/balance.rs"]
mod tests;
