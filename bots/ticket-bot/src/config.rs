pub struct Config {
    pub discord_token: String,
    pub api_base_url: String,
    pub api_key: String,
    pub ticket_category_id: Option<u64>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            discord_token: std::env::var("DISCORD_TOKEN")
                .expect("DISCORD_TOKEN manquant dans .env"),
            api_base_url: std::env::var("API_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            api_key: std::env::var("API_KEY").unwrap_or_default(),
            ticket_category_id: std::env::var("TICKET_CATEGORY_ID")
                .ok()
                .and_then(|v| v.parse().ok()),
        }
    }
}
