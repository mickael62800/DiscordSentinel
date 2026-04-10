/// Phase 6A — Intervalle de scan des jobs d'export pending.
/// 5 secondes : equilibre entre reactivite (le client attend sa piece jointe)
/// et charge DB (poll loop).
const DEFAULT_SCAN_INTERVAL_SECS: u64 = 5;

/// Timeout au-dela duquel un job 'processing' est considere zombie et reset
/// a 'pending' pour retry (protection contre crash worker en plein job).
pub const PROCESSING_TIMEOUT_SECS: i64 = 300;

/// Nombre max de lignes par export (garde-fou memoire — 50k lignes en JSON
/// font environ 20-50 MB selon la richesse, au-dela c'est plus sage de
/// passer par un storage externe).
pub const MAX_ROWS_PER_EXPORT: i64 = 50_000;

pub struct WorkerConfig {
    pub database_url: String,
    pub api_url: String,
    pub scan_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        use sentinel_worker_common::{load_api_url, load_database_url, load_env};

        Self {
            database_url: load_database_url(),
            api_url: load_api_url(),
            scan_interval_secs: load_env("EXPORT_SCAN_INTERVAL", DEFAULT_SCAN_INTERVAL_SECS),
        }
    }

    pub fn apply_db_config(&mut self, db: &std::collections::HashMap<String, String>) {
        use sentinel_worker_common::config_or_env;
        self.scan_interval_secs = config_or_env(
            db,
            "export_scan_interval",
            "EXPORT_SCAN_INTERVAL",
            DEFAULT_SCAN_INTERVAL_SECS,
        );
    }
}
