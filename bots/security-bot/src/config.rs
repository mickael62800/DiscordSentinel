#[derive(Clone)]
pub struct Config {
    pub discord_token: String,
    pub api_base_url: String,
    pub api_key: String,
    pub raid_join_threshold: u64,
    pub raid_join_window_secs: u64,
    pub min_account_age_secs: u64,
    // Anti-raid avancé
    pub quarantine_role_id: Option<u64>,
    pub quarantine_enabled: bool,
    pub slowmode_seconds: u16,
    pub slowmode_duration_secs: u64,
    pub captcha_enabled: bool,
    pub captcha_timeout_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            discord_token: std::env::var("DISCORD_TOKEN")
                .expect("DISCORD_TOKEN manquant dans .env"),
            api_base_url: std::env::var("API_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            api_key: std::env::var("API_KEY").unwrap_or_default(),
            raid_join_threshold: std::env::var("RAID_JOIN_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            raid_join_window_secs: std::env::var("RAID_JOIN_WINDOW_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            min_account_age_secs: std::env::var("MIN_ACCOUNT_AGE_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(86400),
            quarantine_role_id: std::env::var("QUARANTINE_ROLE_ID")
                .ok()
                .and_then(|v| v.parse().ok()),
            quarantine_enabled: std::env::var("QUARANTINE_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            slowmode_seconds: std::env::var("SLOWMODE_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            slowmode_duration_secs: std::env::var("SLOWMODE_DURATION_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            captcha_enabled: std::env::var("CAPTCHA_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            captcha_timeout_secs: std::env::var("CAPTCHA_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
        }
    }
}
