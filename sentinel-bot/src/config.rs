//! Configuration unifiee du bot sentinel.
//! Lit le token Discord depuis SENTINEL_DISCORD_TOKEN (ou DISCORD_TOKEN fallback).

use crate::shared::config::{BaseConfig, BotConfig};

pub struct Config {
    base: BaseConfig,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            base: BaseConfig::from_env("SENTINEL_DISCORD_TOKEN"),
        }
    }
}

impl BotConfig for Config {
    fn base(&self) -> &BaseConfig {
        &self.base
    }
}
