pub struct WorkerConfig {
    pub database_url: String,
    pub api_url: String,
    pub conduct_regen_interval_secs: u64,
    pub ban_cleanup_interval_secs: u64,
    pub sync_ban_proposals_interval_secs: u64,
    pub send_reminders_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        let regen_hours: u64 = std::env::var("CONDUCT_REGEN_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let cleanup_minutes: u64 = std::env::var("BAN_CLEANUP_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let sync_minutes: u64 = std::env::var("SYNC_BAN_PROPOSALS_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);
        let reminders_secs: u64 = std::env::var("SEND_REMINDERS_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        Self {
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL requis"),
            api_url: std::env::var("API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            conduct_regen_interval_secs: regen_hours * 3600,
            ban_cleanup_interval_secs: cleanup_minutes * 60,
            sync_ban_proposals_interval_secs: sync_minutes * 60,
            send_reminders_interval_secs: reminders_secs,
        }
    }
}
