pub struct Config {
    pub discord_token: String,
    pub api_base_url: String,
    pub api_key: String,
    pub raid_join_threshold: u64,
    pub raid_join_window_secs: u64,
    pub min_account_age_secs: u64,
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
        }
    }
}
