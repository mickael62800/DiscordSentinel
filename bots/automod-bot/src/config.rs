use sentinel_shared::config::{BaseConfig, BotConfig};

/// Configuration du bot chargee depuis les variables d'environnement.
pub struct Config {
    base: BaseConfig,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            base: BaseConfig::from_env("AUTOMOD_DISCORD_TOKEN"),
        }
    }
}

impl BotConfig for Config {
    fn base(&self) -> &BaseConfig {
        &self.base
    }
}
