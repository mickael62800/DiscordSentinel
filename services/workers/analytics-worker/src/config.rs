use sentinel_worker_common::{SECS_PER_HOUR, SECS_PER_MINUTE};

/// Intervalle par defaut pour les snapshots quotidiens (heures).
const DEFAULT_DAILY_INTERVAL_HOURS: u64 = 1;
/// Intervalle par defaut pour les snapshots horaires (minutes).
const DEFAULT_HOURLY_INTERVAL_MINUTES: u64 = 60;

pub struct WorkerConfig {
    pub database_url: String,
    pub api_url: String,
    pub daily_snapshot_interval_secs: u64,
    pub hourly_snapshot_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        use sentinel_worker_common::{load_database_url, load_api_url, load_env};

        let daily_hours: u64 = load_env("DAILY_SNAPSHOT_INTERVAL", DEFAULT_DAILY_INTERVAL_HOURS);
        let hourly_minutes: u64 = load_env("HOURLY_SNAPSHOT_INTERVAL", DEFAULT_HOURLY_INTERVAL_MINUTES);

        Self {
            database_url: load_database_url(),
            api_url: load_api_url(),
            daily_snapshot_interval_secs: daily_hours * SECS_PER_HOUR,
            hourly_snapshot_interval_secs: hourly_minutes * SECS_PER_MINUTE,
        }
    }

    /// Recharge les intervalles depuis la config DB (prioritaire sur env/defaut).
    pub fn apply_db_config(&mut self, db: &std::collections::HashMap<String, String>) {
        use sentinel_worker_common::config_or_env;
        let daily_h: u64 = config_or_env(db, "daily_snapshot_interval", "DAILY_SNAPSHOT_INTERVAL", DEFAULT_DAILY_INTERVAL_HOURS);
        let hourly_m: u64 = config_or_env(db, "hourly_snapshot_interval", "HOURLY_SNAPSHOT_INTERVAL", DEFAULT_HOURLY_INTERVAL_MINUTES);
        self.daily_snapshot_interval_secs = daily_h * SECS_PER_HOUR;
        self.hourly_snapshot_interval_secs = hourly_m * SECS_PER_MINUTE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_intervals() {
        assert_eq!(DEFAULT_DAILY_INTERVAL_HOURS, 1);
        assert_eq!(DEFAULT_HOURLY_INTERVAL_MINUTES, 60);
    }

    #[test]
    fn daily_in_seconds() {
        assert_eq!(DEFAULT_DAILY_INTERVAL_HOURS * SECS_PER_HOUR, 3600);
    }

    #[test]
    fn hourly_in_seconds() {
        assert_eq!(DEFAULT_HOURLY_INTERVAL_MINUTES * SECS_PER_MINUTE, 3600);
    }
}
