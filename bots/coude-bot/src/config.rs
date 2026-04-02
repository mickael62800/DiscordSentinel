use sentinel_shared::config::{BaseConfig, BotConfig};

pub struct Config {
    base: BaseConfig,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            base: BaseConfig::from_env("COUDE_DISCORD_TOKEN"),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| panic!("DATABASE_URL manquant dans .env")),
        }
    }
}

impl BotConfig for Config {
    fn base(&self) -> &BaseConfig {
        &self.base
    }
}
