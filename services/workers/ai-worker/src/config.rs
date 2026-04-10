use sentinel_worker_common::SECS_PER_MINUTE;

/// Phase 4 A — Intervalle de polling de la table ai_jobs. 2 secondes : assez
/// bas pour rester reactif (les bots attendent une analyse rapide), assez
/// haut pour ne pas marteler la DB.
const DEFAULT_AI_POLL_SECS: u64 = 2;

/// Timeout d'un job en cours : si un job reste 'processing' plus longtemps,
/// on le remet en 'pending' (le worker a probablement crash).
const DEFAULT_AI_JOB_TIMEOUT_SECS: u64 = 2 * SECS_PER_MINUTE;

pub struct WorkerConfig {
    pub database_url: String,
    pub redis_url: String,
    pub api_url: String,
    pub poll_interval_secs: u64,
    pub job_timeout_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        use sentinel_worker_common::{load_api_url, load_database_url, load_env, load_redis_url};

        Self {
            database_url: load_database_url(),
            redis_url: load_redis_url(),
            api_url: load_api_url(),
            poll_interval_secs: load_env("AI_POLL_INTERVAL", DEFAULT_AI_POLL_SECS),
            job_timeout_secs: load_env("AI_JOB_TIMEOUT", DEFAULT_AI_JOB_TIMEOUT_SECS),
        }
    }

    pub fn apply_db_config(&mut self, db: &std::collections::HashMap<String, String>) {
        use sentinel_worker_common::config_or_env;
        self.poll_interval_secs = config_or_env(db, "ai_poll_interval", "AI_POLL_INTERVAL", DEFAULT_AI_POLL_SECS);
        self.job_timeout_secs = config_or_env(db, "ai_job_timeout", "AI_JOB_TIMEOUT", DEFAULT_AI_JOB_TIMEOUT_SECS);
    }
}
