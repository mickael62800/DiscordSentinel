pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub api_key: String,
    pub host: String,
    pub port: u16,
    /// Phase 7A — port d'ecoute du serveur gRPC interne (tonic).
    /// Coexiste avec le port HTTP/Axum. Defaut : 50051.
    pub grpc_port: u16,
    pub rate_limit_per_sec: u64,
    pub max_body_size: usize,
    pub shutdown_timeout_secs: u64,
    /// Comma-separated list of allowed origins. Empty or "*" = permissive (dev mode).
    pub allowed_origins: String,
    /// Discord bot token pour executer des bans (optionnel).
    pub discord_bot_token: String,
    /// Phase 7 B — Liste de Discord user IDs "superadmin" autorises sur les
    /// endpoints globaux (non scoped par guild). Format : comma-separated.
    /// Ex: `SUPERADMIN_USER_IDS=123456789012345678,234567890123456789`
    pub superadmin_user_ids: Vec<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL manquant"),
            redis_url: std::env::var("REDIS_URL")
                .expect("REDIS_URL manquant"),
            api_key: {
                let key = std::env::var("API_KEY").unwrap_or_default();
                let require = std::env::var("REQUIRE_API_KEY")
                    .map(|v| v != "false" && v != "0")
                    .unwrap_or(true);
                if key.is_empty() && require {
                    tracing::error!("API_KEY non configuree. Definir API_KEY ou REQUIRE_API_KEY=false pour le dev.");
                    std::process::exit(1);
                }
                if !key.is_empty() && key.len() < 16 {
                    tracing::warn!("API_KEY trop courte ({} chars). Utiliser au moins 32 chars en production.", key.len());
                }
                key
            },
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .expect("PORT invalide"),
            grpc_port: std::env::var("GRPC_PORT")
                .unwrap_or_else(|_| "50051".into())
                .parse()
                .unwrap_or(50051),
            rate_limit_per_sec: std::env::var("RATE_LIMIT_PER_SEC")
                .unwrap_or_else(|_| "200".into())
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
            discord_bot_token: std::env::var("AUTOMOD_DISCORD_TOKEN")
                .or_else(|_| std::env::var("MODERATION_DISCORD_TOKEN"))
                .unwrap_or_default(),
            superadmin_user_ids: std::env::var("SUPERADMIN_USER_IDS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn grpc_bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.grpc_port)
    }
}
