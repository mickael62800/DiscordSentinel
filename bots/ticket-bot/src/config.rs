use serenity::prelude::TypeMapKey;
use sentinel_shared::config::{BaseConfig, BotConfig};

#[derive(Clone)]
#[allow(dead_code)]
pub struct Config {
    base: BaseConfig,
    pub ticket_category_id: Option<u64>,
    pub ticket_channel_id: Option<u64>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            base: BaseConfig::from_env("TICKET_DISCORD_TOKEN"),
            ticket_category_id: std::env::var("TICKET_CATEGORY_ID")
                .ok()
                .and_then(|v| v.parse().ok()),
            ticket_channel_id: std::env::var("TICKET_CHANNEL_ID")
                .ok()
                .and_then(|v| v.parse().ok()),
        }
    }
}

impl BotConfig for Config {
    fn base(&self) -> &BaseConfig {
        &self.base
    }
}

pub struct ConfigKey;

impl TypeMapKey for ConfigKey {
    type Value = Config;
}
