pub struct WorkerConfig {
    pub database_url: String,
    pub api_url: String,
    pub daily_snapshot_interval_secs: u64,
    pub hourly_snapshot_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        let daily_hours: u64 = std::env::var("DAILY_SNAPSHOT_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let hourly_minutes: u64 = std::env::var("HOURLY_SNAPSHOT_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        Self {
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL requis"),
            api_url: std::env::var("API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            daily_snapshot_interval_secs: daily_hours * 3600,
            hourly_snapshot_interval_secs: hourly_minutes * 60,
        }
    }
}
