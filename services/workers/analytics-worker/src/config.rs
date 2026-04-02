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
        let daily_hours: u64 = std::env::var("DAILY_SNAPSHOT_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_DAILY_INTERVAL_HOURS);
        let hourly_minutes: u64 = std::env::var("HOURLY_SNAPSHOT_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_HOURLY_INTERVAL_MINUTES);

        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| {
                    tracing::error!("DATABASE_URL non defini");
                    std::process::exit(1);
                }),
            api_url: std::env::var("API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            daily_snapshot_interval_secs: daily_hours * SECS_PER_HOUR,
            hourly_snapshot_interval_secs: hourly_minutes * SECS_PER_MINUTE,
        }
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
