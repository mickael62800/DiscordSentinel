use sentinel_worker_common::{SECS_PER_HOUR, SECS_PER_MINUTE};

/// Intervalle par defaut pour le rafraichissement du cache analytics (secondes).
const DEFAULT_ANALYTICS_REFRESH_SECS: u64 = 5 * SECS_PER_MINUTE;
/// Intervalle par defaut pour le rafraichissement du cache dashboard (secondes).
const DEFAULT_DASHBOARD_REFRESH_SECS: u64 = 10 * SECS_PER_MINUTE;
/// Intervalle par defaut pour le rafraichissement du cache voice stats (secondes).
const DEFAULT_VOICE_STATS_REFRESH_SECS: u64 = 1 * SECS_PER_HOUR;

pub struct WorkerConfig {
    pub database_url: String,
    pub redis_url: String,
    pub api_url: String,
    pub analytics_refresh_secs: u64,
    pub dashboard_refresh_secs: u64,
    pub voice_stats_refresh_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        let analytics_refresh_secs: u64 = std::env::var("ANALYTICS_CACHE_REFRESH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_ANALYTICS_REFRESH_SECS);

        let dashboard_refresh_secs: u64 = std::env::var("DASHBOARD_CACHE_REFRESH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_DASHBOARD_REFRESH_SECS);

        let voice_stats_refresh_secs: u64 = std::env::var("VOICE_STATS_CACHE_REFRESH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_VOICE_STATS_REFRESH_SECS);

        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| {
                    tracing::error!("DATABASE_URL non defini");
                    std::process::exit(1);
                }),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
            api_url: std::env::var("API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            analytics_refresh_secs,
            dashboard_refresh_secs,
            voice_stats_refresh_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_analytics_refresh() {
        assert_eq!(DEFAULT_ANALYTICS_REFRESH_SECS, 300);
    }

    #[test]
    fn default_dashboard_refresh() {
        assert_eq!(DEFAULT_DASHBOARD_REFRESH_SECS, 600);
    }

    #[test]
    fn default_voice_stats_refresh() {
        assert_eq!(DEFAULT_VOICE_STATS_REFRESH_SECS, 3600);
    }

    #[test]
    fn analytics_in_minutes() {
        assert_eq!(DEFAULT_ANALYTICS_REFRESH_SECS / SECS_PER_MINUTE, 5);
    }

    #[test]
    fn dashboard_in_minutes() {
        assert_eq!(DEFAULT_DASHBOARD_REFRESH_SECS / SECS_PER_MINUTE, 10);
    }

    #[test]
    fn voice_stats_in_hours() {
        assert_eq!(DEFAULT_VOICE_STATS_REFRESH_SECS / SECS_PER_HOUR, 1);
    }
}
