use sentinel_shared::config::{BaseConfig, BotConfig, load_env};

/// Configuration du bot chargee depuis les variables d'environnement.
pub struct Config {
    base: BaseConfig,
    /// Taille max d'image acceptee en bytes (defaut: 10 MB)
    pub max_image_size: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            base: BaseConfig::from_env("IMAGE_DISCORD_TOKEN"),
            max_image_size: load_env("MAX_IMAGE_SIZE", 10 * 1024 * 1024),
        }
    }
}

impl BotConfig for Config {
    fn base(&self) -> &BaseConfig {
        &self.base
    }
}
