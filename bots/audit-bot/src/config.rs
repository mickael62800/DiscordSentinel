use sentinel_shared::config::{BaseConfig, BotConfig};

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
            message_cache_size: std::env::var("MESSAGE_CACHE_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10_000),
            anomaly_window_secs: std::env::var("ANOMALY_WINDOW_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            anomaly_mass_ban_threshold: std::env::var("ANOMALY_MASS_BAN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            anomaly_mass_delete_threshold: std::env::var("ANOMALY_MASS_DELETE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(20),
            anomaly_mass_role_threshold: std::env::var("ANOMALY_MASS_ROLE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            weekly_report_enabled: std::env::var("WEEKLY_REPORT_ENABLED")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true),
        }
    }
}

impl BotConfig for Config {
    fn base(&self) -> &BaseConfig {
        &self.base
    }
}
