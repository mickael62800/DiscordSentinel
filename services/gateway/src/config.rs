/// Configuration du gateway chargee depuis les variables d'environnement.
pub struct Config {
    pub host: String,
    pub port: u16,
    pub redis_url: String,
    pub api_key: String,
    pub redis_channel: String,
    pub allowed_origins: String,
    pub max_connections: usize,
    pub api_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3001),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            api_key: std::env::var("API_KEY").unwrap_or_default(),
            redis_channel: std::env::var("REDIS_CHANNEL")
                .unwrap_or_else(|_| "sentinel:events".to_string()),
            allowed_origins: std::env::var("ALLOWED_ORIGINS").unwrap_or_default(),
            max_connections: std::env::var("MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            api_url: std::env::var("API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
