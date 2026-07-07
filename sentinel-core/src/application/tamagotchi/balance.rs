//! Balance (regles de jeu) du Tamagotchi : couts, deltas, recompenses,
//! cooldowns et poids de combat.
//!
//! SOURCE UNIQUE server-side : les valeurs sont lues depuis `bot_guild_config`
//! (composant `tamagotchi-bot`) via le port `BotConfigRepository`, avec des
//! DEFAUTS identiques a ceux historiquement codes dans le bot. Le bot n'envoie
//! plus que l'action + les identifiants ; l'API calcule tout ici.

use crate::domain::entities::system::bot_config::BotGuildConfig;

/// Composant de config (bot_name) ou vivent les reglages du tamagotchi.
pub const BOT_NAME: &str = "tamagotchi-bot";

/// Vue typee de la config balance d'une guild (defauts = valeurs bot).
pub struct TamaBalance {
    entries: Vec<BotGuildConfig>,
}

/// Effets/couts calcules d'une action de soin ou d'un achat boutique.
#[derive(Debug, Clone)]
pub struct CareEffect {
    pub coin_cost: i64,
    pub hunger_delta: i32,
    pub happiness_delta: i32,
    pub energy_delta: i32,
    pub xp_gain: i64,
    pub cooldown_secs: i64,
    pub cure: bool,
}

/// Regles d'un entrainement de stat.
#[derive(Debug, Clone)]
pub struct TrainEffect {
    pub energy_cost: i32,
    pub coin_cost: i64,
    pub stat_gain: i32,
    pub cooldown_secs: i64,
}

/// Recompenses/limites d'une visite.
#[derive(Debug, Clone)]
pub struct VisitEffect {
    pub xp_reward: i64,
    pub coins_reward: i64,
    pub cooldown_secs: i64,
    pub max_per_day: i64,
}

/// Regles d'un combat (cout, cooldown, poids, ELO, XP, alea).
#[derive(Debug, Clone)]
pub struct CombatEffect {
    pub energy_cost: i32,
    pub cooldown_secs: i64,
    pub elo_k: i32,
    pub xp_win: i64,
    pub xp_loss: i64,
    pub w_str: i32,
    pub w_vit: i32,
    pub w_agi: i32,
    pub random_max: i32,
}

impl TamaBalance {
    pub fn new(entries: Vec<BotGuildConfig>) -> Self {
        Self { entries }
    }

    /// Lit une valeur entiere (u64) ; defaut si absente ou non parsable.
    /// Parite exacte avec `BaseApiClient::config_u64` cote bot.
    fn u64(&self, key: &str, default: u64) -> u64 {
        self.entries
            .iter()
            .find(|e| e.config_key == key)
            .and_then(|e| e.config_value.parse().ok())
            .unwrap_or(default)
    }

    /// XP gagne par action de soin (feed/play/cuddle).
    fn xp_per_action(&self) -> i64 {
        self.u64("xp_per_action", 5) as i64
    }

    /// Effets d'une action de soin OU d'un achat boutique. `None` si l'action
    /// est inconnue (le service la rejette). Les gains boutique sont clampes a
    /// 100 (parite bot : une jauge va de 0 a 100).
    pub fn care_effect(&self, action: &str) -> Option<CareEffect> {
        let xp = self.xp_per_action();
        let gain = |k: &str, d: u64| self.u64(k, d).min(100) as i32;
        let cost = |k: &str, d: u64| self.u64(k, d) as i64;
        let effect = match action {
            "feed" => CareEffect {
                coin_cost: cost("feed_cost", 20),
                hunger_delta: self.u64("feed_hunger_gain", 40) as i32,
                happiness_delta: 0,
                energy_delta: 0,
                xp_gain: xp,
                cooldown_secs: self.u64("feed_cooldown_secs", 1800) as i64,
                cure: false,
            },
            "play" => CareEffect {
                coin_cost: 0,
                hunger_delta: 0,
                happiness_delta: self.u64("play_happiness_gain", 30) as i32,
                energy_delta: -(self.u64("play_energy_cost", 10) as i32),
                xp_gain: xp,
                cooldown_secs: self.u64("play_cooldown_secs", 1800) as i64,
                cure: false,
            },
            "sleep" => CareEffect {
                coin_cost: 0,
                hunger_delta: 0,
                happiness_delta: 0,
                energy_delta: self.u64("sleep_energy_gain", 60) as i32,
                xp_gain: 0,
                cooldown_secs: self.u64("sleep_cooldown_secs", 1020) as i64,
                cure: false,
            },
            "cuddle" => CareEffect {
                coin_cost: 0,
                hunger_delta: 0,
                happiness_delta: self.u64("cuddle_happiness_gain", 15) as i32,
                energy_delta: 0,
                xp_gain: xp,
                cooldown_secs: self.u64("cuddle_cooldown_secs", 3600) as i64,
                cure: false,
            },
            // Achats boutique : prix + effets configurables, aucun cooldown ni XP.
            "buy_croquettes" => CareEffect {
                coin_cost: cost("shop_croquettes_price", 15),
                hunger_delta: gain("shop_croquettes_hunger_gain", 25),
                happiness_delta: 0,
                energy_delta: 0,
                xp_gain: 0,
                cooldown_secs: 0,
                cure: false,
            },
            "buy_repas" => CareEffect {
                coin_cost: cost("shop_repas_price", 40),
                hunger_delta: gain("shop_repas_hunger_gain", 60),
                happiness_delta: 0,
                energy_delta: 0,
                xp_gain: 0,
                cooldown_secs: 0,
                cure: false,
            },
            "buy_boisson" => CareEffect {
                coin_cost: cost("shop_boisson_price", 25),
                hunger_delta: 0,
                happiness_delta: 0,
                energy_delta: gain("shop_boisson_energy_gain", 40),
                xp_gain: 0,
                cooldown_secs: 0,
                cure: false,
            },
            "buy_jouet" => CareEffect {
                coin_cost: cost("shop_jouet_price", 20),
                hunger_delta: 0,
                happiness_delta: gain("shop_jouet_happiness_gain", 35),
                energy_delta: 0,
                xp_gain: 0,
                cooldown_secs: 0,
                cure: false,
            },
            "buy_potion" => CareEffect {
                coin_cost: cost("shop_potion_price", 100),
                hunger_delta: gain("shop_potion_hunger_gain", 10),
                happiness_delta: gain("shop_potion_happiness_gain", 10),
                energy_delta: gain("shop_potion_energy_gain", 10),
                xp_gain: 0,
                cooldown_secs: 0,
                cure: true,
            },
            _ => return None,
        };
        Some(effect)
    }

    pub fn train_effect(&self) -> TrainEffect {
        TrainEffect {
            energy_cost: self.u64("train_energy_cost", 25) as i32,
            coin_cost: self.u64("train_cost", 0) as i64,
            stat_gain: self.u64("train_stat_gain", 1) as i32,
            cooldown_secs: self.u64("train_cooldown_secs", 7200) as i64,
        }
    }

    pub fn visit_effect(&self) -> VisitEffect {
        VisitEffect {
            xp_reward: self.u64("visit_xp_reward", 5) as i64,
            coins_reward: self.u64("visit_coins_reward", 5) as i64,
            cooldown_secs: self.u64("visit_cooldown_secs", 6600) as i64,
            max_per_day: self.u64("visit_max_per_day", 10) as i64,
        }
    }

    pub fn combat_effect(&self) -> CombatEffect {
        CombatEffect {
            energy_cost: self.u64("combat_energy_cost", 20) as i32,
            cooldown_secs: self.u64("combat_cooldown_secs", 3600) as i64,
            elo_k: self.u64("combat_elo_k", 32) as i32,
            xp_win: self.u64("combat_xp_win", 50) as i64,
            xp_loss: self.u64("combat_xp_loss", 15) as i64,
            w_str: self.u64("combat_w_str", 3) as i32,
            w_vit: self.u64("combat_w_vit", 2) as i32,
            w_agi: self.u64("combat_w_agi", 2) as i32,
            random_max: self.u64("combat_random_max", 30) as i32,
        }
    }
}
