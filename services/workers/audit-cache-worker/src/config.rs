/// Phase 6A — Intervalle de refresh du cache watched_users.
/// 60 secondes : meme cadence que l'ancienne boucle interne au audit-bot.
const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 60;

pub struct WorkerConfig {
    pub database_url: String,
    pub redis_url: String,
    pub api_url: String,
    pub refresh_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        use sentinel_worker_common::{load_api_url, load_database_url, load_env, load_redis_url};

        Self {
            database_url: load_database_url(),
            redis_url: load_redis_url(),
            api_url: load_api_url(),
            refresh_interval_secs: load_env(
                "AUDIT_CACHE_REFRESH_INTERVAL",
                DEFAULT_REFRESH_INTERVAL_SECS,
            ),
        }
    }

    pub fn apply_db_config(&mut self, db: &std::collections::HashMap<String, String>) {
        use sentinel_worker_common::config_or_env;
        self.refresh_interval_secs = config_or_env(
            db,
            "audit_cache_refresh_interval",
            "AUDIT_CACHE_REFRESH_INTERVAL",
            DEFAULT_REFRESH_INTERVAL_SECS,
        );
    }
}
