use sentinel_shared::config::{BaseConfig, BotConfig, load_env, load_env_bool, load_env_optional, load_env_string};

#[derive(Clone)]
pub struct Config {
    base: BaseConfig,
    pub raid_join_threshold: u64,
    pub raid_join_window_secs: u64,
    pub min_account_age_secs: u64,
    // Anti-raid avance
    pub quarantine_role_id: Option<u64>,
    pub quarantine_enabled: bool,
    pub slowmode_seconds: u16,
    pub slowmode_duration_secs: u64,
    pub captcha_enabled: bool,
    pub captcha_timeout_secs: u64,
    pub captcha_type: String,
    // Lockdown
    pub lockdown_enabled: bool,
    pub lockdown_duration_secs: u64,
    // Alt detection
    pub alt_detection_enabled: bool,
    pub alt_retention_secs: u64,
    pub alt_name_distance: usize,
    // Raid pattern analysis
    pub raid_pattern_enabled: bool,
    pub raid_pattern_score_threshold: u32,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            base: BaseConfig::from_env("SECURITY_DISCORD_TOKEN"),
            raid_join_threshold: load_env("RAID_JOIN_THRESHOLD", 10),
            raid_join_window_secs: load_env("RAID_JOIN_WINDOW_SECS", 10),
            min_account_age_secs: load_env("MIN_ACCOUNT_AGE_SECS", 86400),
            quarantine_role_id: load_env_optional("QUARANTINE_ROLE_ID"),
            quarantine_enabled: load_env_bool("QUARANTINE_ENABLED", false),
            slowmode_seconds: load_env("SLOWMODE_SECONDS", 10),
            slowmode_duration_secs: load_env("SLOWMODE_DURATION_SECS", 300),
            captcha_enabled: load_env_bool("CAPTCHA_ENABLED", false),
            captcha_timeout_secs: load_env("CAPTCHA_TIMEOUT_SECS", 300),
            captcha_type: {
                let ct = load_env_string("CAPTCHA_TYPE", "button");
                if ct != "button" && ct != "math" {
                    tracing::warn!(value=%ct, "CAPTCHA_TYPE invalide, utilisation de 'button' par defaut");
                    "button".to_string()
                } else {
                    ct
                }
            },
            lockdown_enabled: load_env_bool("LOCKDOWN_ENABLED", false),
            lockdown_duration_secs: load_env("LOCKDOWN_DURATION_SECS", 300),
            alt_detection_enabled: load_env_bool("ALT_DETECTION_ENABLED", false),
            alt_retention_secs: load_env("ALT_RETENTION_SECS", 604_800),
            alt_name_distance: load_env("ALT_NAME_DISTANCE", 2),
            raid_pattern_enabled: load_env_bool("RAID_PATTERN_ENABLED", true),
            raid_pattern_score_threshold: load_env("RAID_PATTERN_SCORE_THRESHOLD", 60),
        }
    }
}

impl BotConfig for Config {
    fn base(&self) -> &BaseConfig {
        &self.base
    }
}
