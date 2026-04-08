use sentinel_shared::config::{BaseConfig, BotConfig, load_env, load_env_bool};

#[derive(Clone)]
pub struct Config {
    base: BaseConfig,
    pub message_cache_size: usize,
    pub anomaly_window_secs: u64,
    pub anomaly_mass_ban_threshold: usize,
    pub anomaly_mass_delete_threshold: usize,
    pub anomaly_mass_role_threshold: usize,
    pub weekly_report_enabled: bool,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            base: BaseConfig::from_env("AUDIT_DISCORD_TOKEN"),
            message_cache_size: load_env("MESSAGE_CACHE_SIZE", 10_000),
            anomaly_window_secs: load_env("ANOMALY_WINDOW_SECS", 60),
            anomaly_mass_ban_threshold: load_env("ANOMALY_MASS_BAN", 5),
            anomaly_mass_delete_threshold: load_env("ANOMALY_MASS_DELETE", 20),
            anomaly_mass_role_threshold: load_env("ANOMALY_MASS_ROLE", 10),
            weekly_report_enabled: load_env_bool("WEEKLY_REPORT_ENABLED", true),
        }
    }
}

impl BotConfig for Config {
    fn base(&self) -> &BaseConfig {
        &self.base
    }
}
