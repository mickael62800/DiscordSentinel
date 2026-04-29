//! Helper de lecture des config_keys du bot 'coude-bot' pour une guild.
//!
//! Reproduit le pattern de `BaseApiClient::config_u64/_bool` cote bot, mais
//! cote API : on charge la liste des entrees `bot_guild_config` une fois
//! puis on lit les cles avec un default. Aucune erreur ne fait planter le
//! caller — on retombe systematiquement sur le default.
//!
//! Utilise par les services qui exposent des params via la migration 170
//! (lucky shield, prestige, friendly duel, assurance level, safety net).

use std::collections::HashMap;

use crate::domain::entities::coude::balance::BalanceParams;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

const BOT_NAME: &str = "coude-bot";

/// Charge les `BalanceParams` d'une guild depuis le `BotConfigRepository`.
///
/// API P0 #3 audit : centralise la duplication qui existait dans
/// `manage_coude_combats_service`, `manage_coude_heist_service`,
/// `manage_coude_steal_boosts_service`, `resolve_betting_batch_service`,
/// `resolve_combat_now_service`. Fallback silencieux sur `default()` si
/// le repo est down — les services qui ont besoin d'un comportement
/// strict doivent gerer les erreurs avant d'appeler.
pub async fn load_balance_params(
    repo: &dyn BotConfigRepository,
    guild_id: &str,
) -> BalanceParams {
    match repo.get_config(guild_id, BOT_NAME).await {
        Ok(entries) => {
            let map: HashMap<String, String> = entries
                .into_iter()
                .map(|e| (e.config_key, e.config_value))
                .collect();
            BalanceParams::from_config(&map)
        }
        Err(_) => BalanceParams::default(),
    }
}

#[derive(Debug, Default, Clone)]
pub struct GuildSettings {
    raw: HashMap<String, String>,
}

impl GuildSettings {
    pub async fn load(repo: &dyn BotConfigRepository, guild_id: &str) -> Self {
        match repo.get_config(guild_id, BOT_NAME).await {
            Ok(entries) => Self {
                raw: entries
                    .into_iter()
                    .map(|e| (e.config_key, e.config_value))
                    .collect(),
            },
            Err(_) => Self::default(),
        }
    }

    pub fn get_i64(&self, key: &str, default: i64) -> i64 {
        self.raw
            .get(key)
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(default)
    }

    pub fn get_i32(&self, key: &str, default: i32) -> i32 {
        self.raw
            .get(key)
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(default)
    }

    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.raw
            .get(key)
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on"))
            .unwrap_or(default)
    }

    /// Lit un pourcentage stocke en entier (ex. "50" pour 50%) et le
    /// retourne en `f64` ratio (0.5).
    pub fn get_percent_ratio(&self, key: &str, default_pct: i64) -> f64 {
        (self.get_i64(key, default_pct) as f64) / 100.0
    }
}
