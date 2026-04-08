use serenity::prelude::TypeMapKey;
use sentinel_shared::config::{BaseConfig, BotConfig, load_env_optional};

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
            ticket_category_id: load_env_optional("TICKET_CATEGORY_ID"),
            ticket_channel_id: load_env_optional("TICKET_CHANNEL_ID"),
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
