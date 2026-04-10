/// Phase 6A — Intervalle de scan des tables blackjack AFK.
/// 60 secondes : meme cadence que l'ancienne boucle interne au blackjack-bot.
const DEFAULT_SCAN_INTERVAL_SECS: u64 = 60;

/// Timeout d'inactivite (secondes). 30 min par defaut, identique a l'ancien
/// `AFK_TIMEOUT_SECS` du blackjack-bot.
pub const DEFAULT_AFK_TIMEOUT_SECS: i64 = 1800;

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
            scan_interval_secs: load_env("BLACKJACK_CLEANUP_SCAN_INTERVAL", DEFAULT_SCAN_INTERVAL_SECS),
        }
    }

    pub fn apply_db_config(&mut self, db: &std::collections::HashMap<String, String>) {
        use sentinel_worker_common::config_or_env;
        self.scan_interval_secs = config_or_env(
            db,
            "blackjack_cleanup_scan_interval",
            "BLACKJACK_CLEANUP_SCAN_INTERVAL",
            DEFAULT_SCAN_INTERVAL_SECS,
        );
    }
}
