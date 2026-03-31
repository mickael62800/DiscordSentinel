use sentinel_shared::config::{BaseConfig, BotConfig};

pub struct Config {
    base: BaseConfig,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            base: BaseConfig::from_env("PROGRESSION_DISCORD_TOKEN"),
        }
    }
}

impl BotConfig for Config {
    fn base(&self) -> &BaseConfig {
        &self.base
    }
}
