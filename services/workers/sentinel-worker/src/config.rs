//! Config globale du worker unifie.
//!
//! Chaque domaine ajoute ses propres champs ici (intervalles, retentions,
//! flags). Les valeurs viennent par defaut du code, peuvent etre override
//! par variables d'env, et finalement par la table `bot_guild_config`
//! (cle `sentinel-worker`). Pattern aligne sur les anciens workers.

use std::collections::HashMap;

use sentinel_worker_common::{
    config_or_env, load_api_url, load_database_url, load_env, load_env_bool, load_redis_url,
    SECS_PER_HOUR, SECS_PER_MINUTE,
};

// ── Defauts cleanup ──
const DEFAULT_VOICE_SESSIONS_RETENTION_DAYS: i64 = 90;
const DEFAULT_LOGS_RETENTION_DAYS: i64 = 30;
const DEFAULT_CLOSED_TICKETS_RETENTION_DAYS: i64 = 180;
const DEFAULT_CLEANUP_INTERVAL_HOURS: u64 = 1;
const DEFAULT_VACUUM_INTERVAL_HOURS: u64 = 24;

// ── Defauts cache (warm Redis) ──
const DEFAULT_ANALYTICS_REFRESH_SECS: u64 = 5 * SECS_PER_MINUTE;
const DEFAULT_DASHBOARD_REFRESH_SECS: u64 = 10 * SECS_PER_MINUTE;
const DEFAULT_VOICE_STATS_REFRESH_SECS: u64 = SECS_PER_HOUR;
const DEFAULT_LEADERBOARDS_REFRESH_SECS: u64 = 5 * SECS_PER_MINUTE;
const DEFAULT_USER_CACHE_SYNC_SECS: u64 = 15 * SECS_PER_MINUTE;
const DEFAULT_PARTITION_MANAGER_SECS: u64 = 24 * SECS_PER_HOUR;

// ── Defauts audit_cache ──
const DEFAULT_AUDIT_CACHE_REFRESH_SECS: u64 = 60;

// ── Defauts blackjack ──
const DEFAULT_BLACKJACK_SCAN_INTERVAL_SECS: u64 = 60;

#[derive(Clone)]
pub struct WorkerConfig {
    pub database_url: String,
    pub redis_url: String,
    pub api_url: String,

    // ── Cleanup ──
    pub voice_sessions_retention_days: i64,
    pub logs_retention_days: i64,
    pub closed_tickets_retention_days: i64,
    pub cleanup_interval_secs: u64,
    pub vacuum_enabled: bool,
    pub vacuum_interval_secs: u64,

    // ── Cache (warm Redis) ──
    pub analytics_refresh_secs: u64,
    pub dashboard_refresh_secs: u64,
    pub voice_stats_refresh_secs: u64,
    pub leaderboards_refresh_secs: u64,
    pub user_cache_sync_secs: u64,
    pub partition_manager_secs: u64,

    // ── Audit cache ──
    pub audit_cache_refresh_secs: u64,

    // ── Blackjack ──
    pub blackjack_scan_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        let cleanup_hours: u64 = load_env("CLEANUP_INTERVAL_HOURS", DEFAULT_CLEANUP_INTERVAL_HOURS);
        let vacuum_hours: u64 = load_env("VACUUM_INTERVAL_HOURS", DEFAULT_VACUUM_INTERVAL_HOURS);

        Self {
            database_url: load_database_url(),
            redis_url: load_redis_url(),
            api_url: load_api_url(),

            // cleanup
            voice_sessions_retention_days: load_env(
                "VOICE_SESSIONS_RETENTION_DAYS",
                DEFAULT_VOICE_SESSIONS_RETENTION_DAYS,
            ),
            logs_retention_days: load_env("LOGS_RETENTION_DAYS", DEFAULT_LOGS_RETENTION_DAYS),
            closed_tickets_retention_days: load_env(
                "CLOSED_TICKETS_RETENTION_DAYS",
                DEFAULT_CLOSED_TICKETS_RETENTION_DAYS,
            ),
            cleanup_interval_secs: cleanup_hours * SECS_PER_HOUR,
            vacuum_enabled: load_env_bool("VACUUM_ENABLED", true),
            vacuum_interval_secs: vacuum_hours * SECS_PER_HOUR,

            // cache
            analytics_refresh_secs: load_env(
                "ANALYTICS_CACHE_REFRESH",
                DEFAULT_ANALYTICS_REFRESH_SECS,
            ),
            dashboard_refresh_secs: load_env(
                "DASHBOARD_CACHE_REFRESH",
                DEFAULT_DASHBOARD_REFRESH_SECS,
            ),
            voice_stats_refresh_secs: load_env(
                "VOICE_STATS_CACHE_REFRESH",
                DEFAULT_VOICE_STATS_REFRESH_SECS,
            ),
            leaderboards_refresh_secs: load_env(
                "LEADERBOARDS_REFRESH",
                DEFAULT_LEADERBOARDS_REFRESH_SECS,
            ),
            user_cache_sync_secs: load_env("USER_CACHE_SYNC", DEFAULT_USER_CACHE_SYNC_SECS),
            partition_manager_secs: load_env("PARTITION_MANAGER", DEFAULT_PARTITION_MANAGER_SECS),

            // audit_cache
            audit_cache_refresh_secs: load_env(
                "AUDIT_CACHE_REFRESH_INTERVAL",
                DEFAULT_AUDIT_CACHE_REFRESH_SECS,
            ),

            // blackjack
            blackjack_scan_interval_secs: load_env(
                "BLACKJACK_CLEANUP_SCAN_INTERVAL",
                DEFAULT_BLACKJACK_SCAN_INTERVAL_SECS,
            ),
        }
    }

    /// Surcharge depuis la table `bot_guild_config` (cle `sentinel-worker`).
    pub fn apply_db_config(&mut self, db: &HashMap<String, String>) {
        // cleanup
        self.voice_sessions_retention_days = config_or_env(
            db,
            "voice_sessions_retention_days",
            "VOICE_SESSIONS_RETENTION_DAYS",
            DEFAULT_VOICE_SESSIONS_RETENTION_DAYS,
        );
        self.logs_retention_days = config_or_env(
            db,
            "logs_retention_days",
            "LOGS_RETENTION_DAYS",
            DEFAULT_LOGS_RETENTION_DAYS,
        );
        self.closed_tickets_retention_days = config_or_env(
            db,
            "closed_tickets_retention_days",
            "CLOSED_TICKETS_RETENTION_DAYS",
            DEFAULT_CLOSED_TICKETS_RETENTION_DAYS,
        );
        let cleanup_hours: u64 = config_or_env(
            db,
            "cleanup_interval_hours",
            "CLEANUP_INTERVAL_HOURS",
            DEFAULT_CLEANUP_INTERVAL_HOURS,
        );
        self.cleanup_interval_secs = cleanup_hours * SECS_PER_HOUR;
        let vacuum_hours: u64 = config_or_env(
            db,
            "vacuum_interval_hours",
            "VACUUM_INTERVAL_HOURS",
            DEFAULT_VACUUM_INTERVAL_HOURS,
        );
        self.vacuum_interval_secs = vacuum_hours * SECS_PER_HOUR;

        // cache
        self.analytics_refresh_secs = config_or_env(
            db,
            "analytics_cache_refresh",
            "ANALYTICS_CACHE_REFRESH",
            DEFAULT_ANALYTICS_REFRESH_SECS,
        );
        self.dashboard_refresh_secs = config_or_env(
            db,
            "dashboard_cache_refresh",
            "DASHBOARD_CACHE_REFRESH",
            DEFAULT_DASHBOARD_REFRESH_SECS,
        );
        self.voice_stats_refresh_secs = config_or_env(
            db,
            "voice_stats_cache_refresh",
            "VOICE_STATS_CACHE_REFRESH",
            DEFAULT_VOICE_STATS_REFRESH_SECS,
        );
        self.leaderboards_refresh_secs = config_or_env(
            db,
            "leaderboards_refresh",
            "LEADERBOARDS_REFRESH",
            DEFAULT_LEADERBOARDS_REFRESH_SECS,
        );
        self.user_cache_sync_secs = config_or_env(
            db,
            "user_cache_sync",
            "USER_CACHE_SYNC",
            DEFAULT_USER_CACHE_SYNC_SECS,
        );
        self.partition_manager_secs = config_or_env(
            db,
            "partition_manager",
            "PARTITION_MANAGER",
            DEFAULT_PARTITION_MANAGER_SECS,
        );

        // audit_cache
        self.audit_cache_refresh_secs = config_or_env(
            db,
            "audit_cache_refresh_interval",
            "AUDIT_CACHE_REFRESH_INTERVAL",
            DEFAULT_AUDIT_CACHE_REFRESH_SECS,
        );

        // blackjack
        self.blackjack_scan_interval_secs = config_or_env(
            db,
            "blackjack_cleanup_scan_interval",
            "BLACKJACK_CLEANUP_SCAN_INTERVAL",
            DEFAULT_BLACKJACK_SCAN_INTERVAL_SECS,
        );
    }
}

/// Sous-config passee aux jobs cleanup (pour ne pas leur donner toute la
/// WorkerConfig).
#[derive(Clone)]
pub struct CleanupConfig {
    pub voice_sessions_retention_days: i64,
    pub logs_retention_days: i64,
    pub closed_tickets_retention_days: i64,
}

impl From<&WorkerConfig> for CleanupConfig {
    fn from(c: &WorkerConfig) -> Self {
        Self {
            voice_sessions_retention_days: c.voice_sessions_retention_days,
            logs_retention_days: c.logs_retention_days,
            closed_tickets_retention_days: c.closed_tickets_retention_days,
        }
    }
}
