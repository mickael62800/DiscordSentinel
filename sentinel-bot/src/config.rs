//! Configuration unifiee du bot sentinel.
//! Lit le token Discord depuis SENTINEL_DISCORD_TOKEN (ou DISCORD_TOKEN fallback).

use crate::shared::config::{BaseConfig, BotConfig};

pub struct Config {
    base: BaseConfig,
}

impl Config {
    pub fn from_env() -> Self {
        // Tente SENTINEL_DISCORD_TOKEN d'abord, puis DISCORD_TOKEN
        let token_key = if std::env::var("SENTINEL_DISCORD_TOKEN").is_ok() {
            "SENTINEL_DISCORD_TOKEN"
        } else {
            "DISCORD_TOKEN"
        };
        Self {
            base: BaseConfig::from_env(token_key),
        }
    }
}

impl BotConfig for Config {
    fn base(&self) -> &BaseConfig {
        &self.base
    }
}
