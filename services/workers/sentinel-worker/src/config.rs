//! Config globale du worker unifie.
//!
//! Chaque domaine ajoute ses propres champs ici (intervalles, retentions,
//! flags). Les valeurs viennent par defaut du code, peuvent etre override
//! par variables d'env, et finalement par la table `bot_guild_config`
//! (cle `sentinel-worker`). Pattern aligne sur les anciens workers.

use std::collections::HashMap;

use sentinel_worker_common::{config_or_env, load_api_url, load_database_url, load_env, load_env_bool, SECS_PER_HOUR};

// ── Defauts cleanup ──
const DEFAULT_VOICE_SESSIONS_RETENTION_DAYS: i64 = 90;
const DEFAULT_LOGS_RETENTION_DAYS: i64 = 30;
const DEFAULT_CLOSED_TICKETS_RETENTION_DAYS: i64 = 180;
const DEFAULT_CLEANUP_INTERVAL_HOURS: u64 = 1;
const DEFAULT_VACUUM_INTERVAL_HOURS: u64 = 24;

#[derive(Clone)]
pub struct WorkerConfig {
    pub database_url: String,
    pub api_url: String,

    // Domaine cleanup (porte de l'ancien cleanup-worker).
    pub voice_sessions_retention_days: i64,
    pub logs_retention_days: i64,
    pub closed_tickets_retention_days: i64,
    pub cleanup_interval_secs: u64,
    pub vacuum_enabled: bool,
    pub vacuum_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        let cleanup_hours: u64 = load_env("CLEANUP_INTERVAL_HOURS", DEFAULT_CLEANUP_INTERVAL_HOURS);
        let vacuum_hours: u64 = load_env("VACUUM_INTERVAL_HOURS", DEFAULT_VACUUM_INTERVAL_HOURS);

        Self {
            database_url: load_database_url(),
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
        }
    }

    /// Surcharge depuis la table `bot_guild_config` (cle `sentinel-worker`).
    /// Permet de modifier les intervalles a chaud sans redeployer.
    pub fn apply_db_config(&mut self, db: &HashMap<String, String>) {
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
    }
}

/// Sous-config passee aux jobs cleanup (sous-ensemble pour ne pas leur
/// donner toute la WorkerConfig).
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
