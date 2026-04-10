use sentinel_worker_common::{SECS_PER_HOUR, SECS_PER_MINUTE};

/// Intervalle par defaut pour le rafraichissement du cache analytics (secondes).
const DEFAULT_ANALYTICS_REFRESH_SECS: u64 = 5 * SECS_PER_MINUTE;
/// Intervalle par defaut pour le rafraichissement du cache dashboard (secondes).
const DEFAULT_DASHBOARD_REFRESH_SECS: u64 = 10 * SECS_PER_MINUTE;
/// Intervalle par defaut pour le rafraichissement du cache voice stats (secondes).
const DEFAULT_VOICE_STATS_REFRESH_SECS: u64 = 1 * SECS_PER_HOUR;
/// Phase 2 A.2 — Intervalle de refresh des vues materialisees leaderboards.
/// 5 minutes : compromis entre fraicheur (les ranks bougent vite en periode de
/// raid coude/casino) et cout du REFRESH CONCURRENTLY (qui scanne la table).
const DEFAULT_LEADERBOARDS_REFRESH_SECS: u64 = 5 * SECS_PER_MINUTE;
/// Phase 2 A.2 — Intervalle de sync de la table user_cache. 15 minutes
/// suffit largement : un username Discord ne change presque jamais.
const DEFAULT_USER_CACHE_SYNC_SECS: u64 = 15 * SECS_PER_MINUTE;
/// Phase 2 A.4 — Intervalle de creation des partitions futures. Une fois
/// par jour suffit (idempotent, ne cree que ce qui manque).
const DEFAULT_PARTITION_MANAGER_SECS: u64 = 24 * SECS_PER_HOUR;

pub struct WorkerConfig {
    pub database_url: String,
    pub redis_url: String,
    pub api_url: String,
    pub analytics_refresh_secs: u64,
    pub dashboard_refresh_secs: u64,
    pub voice_stats_refresh_secs: u64,
    pub leaderboards_refresh_secs: u64,
    pub user_cache_sync_secs: u64,
    pub partition_manager_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        use sentinel_worker_common::{load_database_url, load_api_url, load_redis_url, load_env};

        Self {
            database_url: load_database_url(),
            redis_url: load_redis_url(),
            api_url: load_api_url(),
            analytics_refresh_secs: load_env("ANALYTICS_CACHE_REFRESH", DEFAULT_ANALYTICS_REFRESH_SECS),
            dashboard_refresh_secs: load_env("DASHBOARD_CACHE_REFRESH", DEFAULT_DASHBOARD_REFRESH_SECS),
            voice_stats_refresh_secs: load_env("VOICE_STATS_CACHE_REFRESH", DEFAULT_VOICE_STATS_REFRESH_SECS),
            leaderboards_refresh_secs: load_env("LEADERBOARDS_REFRESH", DEFAULT_LEADERBOARDS_REFRESH_SECS),
            user_cache_sync_secs: load_env("USER_CACHE_SYNC", DEFAULT_USER_CACHE_SYNC_SECS),
            partition_manager_secs: load_env("PARTITION_MANAGER", DEFAULT_PARTITION_MANAGER_SECS),
        }
    }

    pub fn apply_db_config(&mut self, db: &std::collections::HashMap<String, String>) {
        use sentinel_worker_common::config_or_env;
        self.analytics_refresh_secs = config_or_env(db, "analytics_cache_refresh", "ANALYTICS_CACHE_REFRESH", DEFAULT_ANALYTICS_REFRESH_SECS);
        self.dashboard_refresh_secs = config_or_env(db, "dashboard_cache_refresh", "DASHBOARD_CACHE_REFRESH", DEFAULT_DASHBOARD_REFRESH_SECS);
        self.voice_stats_refresh_secs = config_or_env(db, "voice_stats_cache_refresh", "VOICE_STATS_CACHE_REFRESH", DEFAULT_VOICE_STATS_REFRESH_SECS);
        self.leaderboards_refresh_secs = config_or_env(db, "leaderboards_refresh", "LEADERBOARDS_REFRESH", DEFAULT_LEADERBOARDS_REFRESH_SECS);
        self.user_cache_sync_secs = config_or_env(db, "user_cache_sync", "USER_CACHE_SYNC", DEFAULT_USER_CACHE_SYNC_SECS);
        self.partition_manager_secs = config_or_env(db, "partition_manager", "PARTITION_MANAGER", DEFAULT_PARTITION_MANAGER_SECS);
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
