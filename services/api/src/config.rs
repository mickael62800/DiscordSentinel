pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub api_key: String,
    pub host: String,
    pub port: u16,
    pub rate_limit_per_sec: u64,
    pub max_body_size: usize,
    pub shutdown_timeout_secs: u64,
    /// Comma-separated list of allowed origins. Empty or "*" = permissive (dev mode).
    pub allowed_origins: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL manquant"),
            redis_url: std::env::var("REDIS_URL")
                .expect("REDIS_URL manquant"),
            api_key: std::env::var("API_KEY").unwrap_or_default(),
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .expect("PORT invalide"),
            rate_limit_per_sec: std::env::var("RATE_LIMIT_PER_SEC")
                .unwrap_or_else(|_| "50".into())
                .parse()
                .unwrap_or(50),
            max_body_size: std::env::var("MAX_BODY_SIZE")
                .unwrap_or_else(|_| "1048576".into())
                .parse()
                .unwrap_or(1_048_576),
            shutdown_timeout_secs: std::env::var("SHUTDOWN_TIMEOUT")
                .unwrap_or_else(|_| "30".into())
                .parse()
                .unwrap_or(30),
            allowed_origins: std::env::var("ALLOWED_ORIGINS").unwrap_or_default(),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
