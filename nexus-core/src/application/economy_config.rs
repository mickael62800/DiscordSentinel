//! Reglages de l'economie et de Coup de Coude, par serveur.
//!
//! Meme patron que `game::config_loader` : on lit les valeurs stockees pour
//! la guilde et on applique les defauts. Ces defauts reproduisent EXACTEMENT
//! le comportement d'avant l'introduction de la configuration — une
//! installation qui ne touche a rien ne voit aucun changement.
//!
//! Le domaine reste PUR : ces structures lui sont passees en DONNEES. Aucune
//! fonction de jeu ne va chercher sa configuration elle-meme, sinon il
//! deviendrait impossible de la tester sans base.

use std::sync::Arc;

use crate::domain::entities::system::bot_config::BotGuildConfig;
use crate::domain::errors::DomainError;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

pub const ECONOMY_BOT: &str = "nexus-economy";
pub const COUDE_BOT: &str = "nexus-coude";

// ── Economie ──

#[derive(Debug, Clone)]
pub struct EconomyConfig {
    pub enabled: bool,
    pub starting_coins: i64,
    pub transfer_enabled: bool,
    pub transfer_min: i64,
    /// 0 = pas de plafond.
    pub transfer_max: i64,
    pub transfer_fee_pct: i64,
    pub wheel_enabled: bool,
    pub wheel_cooldown_hours: i64,
    /// En pourcentage : 100 = gains inchanges.
    pub wheel_payout_multiplier: i64,
    pub leaderboard_size: i64,
}

impl Default for EconomyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            starting_coins: 100,
            transfer_enabled: true,
            transfer_min: 1,
            transfer_max: 0,
            transfer_fee_pct: 0,
            wheel_enabled: true,
            wheel_cooldown_hours: 24,
            wheel_payout_multiplier: 100,
            leaderboard_size: 10,
        }
    }
}

impl EconomyConfig {
    /// Applique le multiplicateur a un gain ou une perte.
    ///
    /// Multiplie AVANT de diviser : l'inverse ecraserait les petits montants
    /// a zero (50 * 120 / 100 = 60, alors que 50 / 100 * 120 = 0 en entier).
    ///
    /// Le signe est conserve : une perte multipliee reste une perte.
    pub fn apply_payout(&self, base: i64) -> i64 {
        if self.wheel_payout_multiplier == 100 {
            return base;
        }
        base.saturating_mul(self.wheel_payout_multiplier) / 100
    }

    /// Frais preleves sur un transfert, arrondis a l'entier inferieur.
    ///
    /// Arrondi vers le BAS : prelever plus que le pourcentage annonce serait
    /// une mauvaise surprise, prelever moins ne lese personne.
    pub fn transfer_fee(&self, amount: i64) -> i64 {
        if self.transfer_fee_pct <= 0 {
            return 0;
        }
        amount.saturating_mul(self.transfer_fee_pct) / 100
    }

    /// Le montant est-il acceptable pour un transfert ?
    pub fn validate_transfer(&self, amount: i64) -> Result<(), String> {
        if !self.transfer_enabled {
            return Err("les transferts sont desactives sur ce serveur".into());
        }
        if amount < self.transfer_min {
            return Err(format!("le minimum est de {} coins", self.transfer_min));
        }
        if self.transfer_max > 0 && amount > self.transfer_max {
            return Err(format!("le maximum est de {} coins", self.transfer_max));
        }
        Ok(())
    }
}

// ── Coup de Coude ──

#[derive(Debug, Clone)]
pub struct CoudeConfig {
    pub enabled: bool,
    pub max_level: i32,
    pub stat_points_per_level: i32,
    pub xp_winner: i32,
    pub xp_loser: i32,
    pub xp_underdog_bonus: i32,
    pub combat_cooldown_minutes: i64,
    pub combat_mise_min: i64,
    /// 0 = pas de plafond.
    pub combat_mise_max: i64,
    pub steal_enabled: bool,
    pub steal_success_pct: u32,
    pub steal_success_pct_fourbe: u32,
    pub steal_gain_pct: i64,
    pub steal_penalty_pct: i64,
    pub steal_cooldown_minutes: i64,
    pub steal_min_victim_coins: i64,
    pub prime_enabled: bool,
    pub prime_min: i64,
    pub prime_max: i64,
    pub bet_enabled: bool,
    pub bet_min: i64,
    pub bet_payout_multiplier: i64,
    pub insurance_enabled: bool,
    pub insurance_cost: i64,
    pub hp_regen_per_hour: i32,
}

impl Default for CoudeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_level: 25,
            stat_points_per_level: 1,
            xp_winner: 10,
            xp_loser: 3,
            xp_underdog_bonus: 5,
            combat_cooldown_minutes: 0,
            combat_mise_min: 10,
            combat_mise_max: 0,
            steal_enabled: true,
            // Les quatre valeurs historiques du service de vol.
            steal_success_pct: 30,
            steal_success_pct_fourbe: 50,
            steal_gain_pct: 20,
            steal_penalty_pct: 15,
            steal_cooldown_minutes: 30,
            steal_min_victim_coins: 10,
            prime_enabled: true,
            prime_min: 50,
            prime_max: 0,
            bet_enabled: true,
            bet_min: 10,
            bet_payout_multiplier: 200,
            insurance_enabled: true,
            insurance_cost: 100,
            hp_regen_per_hour: 5,
        }
    }
}

impl CoudeConfig {
    /// Chance de reussite d'un vol, selon la classe du voleur.
    pub fn steal_chance(&self, is_fourbe: bool) -> u32 {
        if is_fourbe {
            self.steal_success_pct_fourbe
        } else {
            self.steal_success_pct
        }
        // Un taux de 0 ou 100 rendrait le vol inutile ou infaillible : on
        // garde toujours une part de risque des deux cotes.
        .clamp(1, 99)
    }

    /// Montant vole a une victime. Au moins 1 : un vol reussi qui ne rapporte
    /// rien serait indistinguable d'un echec.
    pub fn steal_gain(&self, victim_coins: i64) -> i64 {
        (victim_coins.saturating_mul(self.steal_gain_pct) / 100).max(1)
    }

    /// Penalite subie par un voleur qui echoue.
    pub fn steal_penalty(&self, thief_coins: i64) -> i64 {
        (thief_coins.saturating_mul(self.steal_penalty_pct) / 100).max(1)
    }

    /// Experience gagnee par le vainqueur, bonus compris s'il partait perdant.
    pub fn combat_xp(&self, is_underdog: bool) -> i32 {
        self.xp_winner + if is_underdog { self.xp_underdog_bonus } else { 0 }
    }

    pub fn validate_mise(&self, mise: i64) -> Result<(), String> {
        if mise < self.combat_mise_min {
            return Err(format!("la mise minimum est de {} coins", self.combat_mise_min));
        }
        if self.combat_mise_max > 0 && mise > self.combat_mise_max {
            return Err(format!("la mise maximum est de {} coins", self.combat_mise_max));
        }
        Ok(())
    }
}

// ── Lecture ──

fn find<'a>(items: &'a [BotGuildConfig], key: &str) -> Option<&'a str> {
    items
        .iter()
        .find(|c| c.config_key == key)
        .map(|c| c.config_value.as_str())
}

fn b(items: &[BotGuildConfig], key: &str, default: bool) -> bool {
    match find(items, key) {
        Some("true") | Some("1") => true,
        Some("false") | Some("0") => false,
        _ => default,
    }
}

/// Une valeur illisible retombe sur le defaut plutot que d'echouer : une
/// saisie erronee ne doit pas rendre le jeu injouable.
fn n<T: std::str::FromStr>(items: &[BotGuildConfig], key: &str, default: T) -> T {
    find(items, key)
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

pub async fn load_economy(
    repo: &Arc<dyn BotConfigRepository>,
    guild_id: &str,
) -> Result<EconomyConfig, DomainError> {
    let items = repo.get_config(guild_id, ECONOMY_BOT).await?;
    let d = EconomyConfig::default();
    Ok(EconomyConfig {
        enabled: b(&items, "enabled", d.enabled),
        starting_coins: n(&items, "starting_coins", d.starting_coins),
        transfer_enabled: b(&items, "transfer_enabled", d.transfer_enabled),
        transfer_min: n(&items, "transfer_min", d.transfer_min),
        transfer_max: n(&items, "transfer_max", d.transfer_max),
        transfer_fee_pct: n(&items, "transfer_fee_pct", d.transfer_fee_pct),
        wheel_enabled: b(&items, "wheel_enabled", d.wheel_enabled),
        wheel_cooldown_hours: n(&items, "wheel_cooldown_hours", d.wheel_cooldown_hours),
        wheel_payout_multiplier: n(&items, "wheel_payout_multiplier", d.wheel_payout_multiplier),
        leaderboard_size: n(&items, "leaderboard_size", d.leaderboard_size),
    })
}

pub async fn load_coude(
    repo: &Arc<dyn BotConfigRepository>,
    guild_id: &str,
) -> Result<CoudeConfig, DomainError> {
    let items = repo.get_config(guild_id, COUDE_BOT).await?;
    let d = CoudeConfig::default();
    Ok(CoudeConfig {
        enabled: b(&items, "enabled", d.enabled),
        max_level: n(&items, "max_level", d.max_level),
        stat_points_per_level: n(&items, "stat_points_per_level", d.stat_points_per_level),
        xp_winner: n(&items, "xp_winner", d.xp_winner),
        xp_loser: n(&items, "xp_loser", d.xp_loser),
        xp_underdog_bonus: n(&items, "xp_underdog_bonus", d.xp_underdog_bonus),
        combat_cooldown_minutes: n(&items, "combat_cooldown_minutes", d.combat_cooldown_minutes),
        combat_mise_min: n(&items, "combat_mise_min", d.combat_mise_min),
        combat_mise_max: n(&items, "combat_mise_max", d.combat_mise_max),
        steal_enabled: b(&items, "steal_enabled", d.steal_enabled),
        steal_success_pct: n(&items, "steal_success_pct", d.steal_success_pct),
        steal_success_pct_fourbe: n(
            &items,
            "steal_success_pct_fourbe",
            d.steal_success_pct_fourbe,
        ),
        steal_gain_pct: n(&items, "steal_gain_pct", d.steal_gain_pct),
        steal_penalty_pct: n(&items, "steal_penalty_pct", d.steal_penalty_pct),
        steal_cooldown_minutes: n(&items, "steal_cooldown_minutes", d.steal_cooldown_minutes),
        steal_min_victim_coins: n(&items, "steal_min_victim_coins", d.steal_min_victim_coins),
        prime_enabled: b(&items, "prime_enabled", d.prime_enabled),
        prime_min: n(&items, "prime_min", d.prime_min),
        prime_max: n(&items, "prime_max", d.prime_max),
        bet_enabled: b(&items, "bet_enabled", d.bet_enabled),
        bet_min: n(&items, "bet_min", d.bet_min),
        bet_payout_multiplier: n(&items, "bet_payout_multiplier", d.bet_payout_multiplier),
        insurance_enabled: b(&items, "insurance_enabled", d.insurance_enabled),
        insurance_cost: n(&items, "insurance_cost", d.insurance_cost),
        hp_regen_per_hour: n(&items, "hp_regen_per_hour", d.hp_regen_per_hour),
    })
}

#[cfg(test)]
#[path = "tests/economy_config.rs"]
mod tests;
