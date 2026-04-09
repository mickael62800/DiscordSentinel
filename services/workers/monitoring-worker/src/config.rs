/// Intervalle de check par defaut (secondes).
const DEFAULT_CHECK_INTERVAL_SECS: u64 = 30;

#[derive(Clone)]
pub struct MonitorConfig {
    pub redis_url: String,
    pub api_url: String,
    pub check_interval_secs: u64,
}

impl MonitorConfig {
    pub fn from_env() -> Self {
        use sentinel_worker_common::{load_api_url, load_redis_url, load_env};

        Self {
            redis_url: load_redis_url(),
            api_url: load_api_url(),
            check_interval_secs: load_env("MONITOR_CHECK_INTERVAL", DEFAULT_CHECK_INTERVAL_SECS),
        }
    }

    pub fn apply_db_config(&mut self, db: &std::collections::HashMap<String, String>) {
        use sentinel_worker_common::config_or_env;
        self.check_interval_secs = config_or_env(db, "monitor_check_interval", "MONITOR_CHECK_INTERVAL", DEFAULT_CHECK_INTERVAL_SECS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_check_interval() {
        assert_eq!(DEFAULT_CHECK_INTERVAL_SECS, 30);
    }
}
