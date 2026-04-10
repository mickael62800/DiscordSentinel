/// Phase 6A — Intervalle de scan des tickets d'appel de sanction.
/// 2 minutes : trade-off entre reactivite et charge DB.
const DEFAULT_SCAN_INTERVAL_SECS: u64 = 120;

/// Defaut SLA de premiere reponse (en minutes) si pas override dans bot_guild_config.
/// Aligne sur la valeur par defaut de migration 047 pour ticket-bot.
pub const DEFAULT_SLA_FIRST_RESPONSE_MINUTES: i64 = 30;

/// Defaut SLA d'escalade (en minutes) si pas override.
pub const DEFAULT_SLA_ESCALATION_MINUTES: i64 = 60;

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
            scan_interval_secs: load_env("APPEAL_SLA_SCAN_INTERVAL", DEFAULT_SCAN_INTERVAL_SECS),
        }
    }

    pub fn apply_db_config(&mut self, db: &std::collections::HashMap<String, String>) {
        use sentinel_worker_common::config_or_env;
        self.scan_interval_secs = config_or_env(
            db,
            "appeal_sla_scan_interval",
            "APPEAL_SLA_SCAN_INTERVAL",
            DEFAULT_SCAN_INTERVAL_SECS,
        );
    }
}
