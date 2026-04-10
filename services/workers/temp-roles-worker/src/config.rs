use sentinel_worker_common::SECS_PER_MINUTE;

/// Phase 4 B — Intervalle de scan des roles temporaires expires.
/// 1 minute : meme cadence que l'ancien cleanup interne au community-bot.
const DEFAULT_SCAN_INTERVAL_SECS: u64 = SECS_PER_MINUTE;

pub struct WorkerConfig {
    pub database_url: String,
    pub redis_url: String,
    pub api_url: String,
    pub scan_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        use sentinel_worker_common::{load_api_url, load_database_url, load_env, load_redis_url};

        Self {
            database_url: load_database_url(),
            redis_url: load_redis_url(),
            api_url: load_api_url(),
            scan_interval_secs: load_env("TEMP_ROLES_SCAN_INTERVAL", DEFAULT_SCAN_INTERVAL_SECS),
        }
    }

    pub fn apply_db_config(&mut self, db: &std::collections::HashMap<String, String>) {
        use sentinel_worker_common::config_or_env;
        self.scan_interval_secs = config_or_env(
            db,
            "temp_roles_scan_interval",
            "TEMP_ROLES_SCAN_INTERVAL",
            DEFAULT_SCAN_INTERVAL_SECS,
        );
    }
}
