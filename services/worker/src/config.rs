pub struct WorkerConfig {
    pub database_url: String,
    pub redis_url: String,
    pub queue_key: String,
    pub conduct_regen_interval_secs: u64,
    pub ban_cleanup_interval_secs: u64,
    pub daily_snapshot_interval_secs: u64,
    pub shutdown_timeout_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL requis"),
            redis_url: std::env::var("REDIS_URL")
                .expect("REDIS_URL requis"),
            queue_key: std::env::var("REDIS_QUEUE_KEY")
                .unwrap_or_else(|_| "sentinel:jobs".to_string()),
            conduct_regen_interval_secs: std::env::var("CONDUCT_REGEN_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            ban_cleanup_interval_secs: std::env::var("BAN_CLEANUP_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            daily_snapshot_interval_secs: std::env::var("DAILY_SNAPSHOT_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            shutdown_timeout_secs: std::env::var("SHUTDOWN_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        }
    }
}
