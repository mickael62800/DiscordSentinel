use sentinel_worker_common::{SECS_PER_HOUR};

/// Retention par defaut pour les sessions vocales (jours).
const DEFAULT_VOICE_SESSIONS_RETENTION_DAYS: i64 = 90;
/// Retention par defaut pour les logs (jours).
const DEFAULT_LOGS_RETENTION_DAYS: i64 = 30;
/// Retention par defaut pour les tickets fermes (jours).
const DEFAULT_CLOSED_TICKETS_RETENTION_DAYS: i64 = 180;
/// Intervalle par defaut pour le nettoyage (heures).
const DEFAULT_CLEANUP_INTERVAL_HOURS: u64 = 1;
/// Intervalle par defaut pour le VACUUM (heures).
const DEFAULT_VACUUM_INTERVAL_HOURS: u64 = 24;

#[derive(Clone)]
pub struct WorkerConfig {
    pub database_url: String,
    pub api_url: String,
    pub voice_sessions_retention_days: i64,
    pub logs_retention_days: i64,
    pub closed_tickets_retention_days: i64,
    pub cleanup_interval_secs: u64,
    pub vacuum_enabled: bool,
    pub vacuum_interval_secs: u64,
}

/// Sous-ensemble de la config passe aux jobs de nettoyage.
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

impl WorkerConfig {
    pub fn from_env() -> Self {
        let cleanup_hours: u64 = std::env::var("CLEANUP_INTERVAL_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_CLEANUP_INTERVAL_HOURS);

        let vacuum_hours: u64 = std::env::var("VACUUM_INTERVAL_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_VACUUM_INTERVAL_HOURS);

        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| {
                    tracing::error!("DATABASE_URL non defini");
                    std::process::exit(1);
                }),
            api_url: std::env::var("API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            voice_sessions_retention_days: std::env::var("VOICE_SESSIONS_RETENTION_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_VOICE_SESSIONS_RETENTION_DAYS),
            logs_retention_days: std::env::var("LOGS_RETENTION_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_LOGS_RETENTION_DAYS),
            closed_tickets_retention_days: std::env::var("CLOSED_TICKETS_RETENTION_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_CLOSED_TICKETS_RETENTION_DAYS),
            cleanup_interval_secs: cleanup_hours * SECS_PER_HOUR,
            vacuum_enabled: std::env::var("VACUUM_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            vacuum_interval_secs: vacuum_hours * SECS_PER_HOUR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_retention_days() {
        assert_eq!(DEFAULT_VOICE_SESSIONS_RETENTION_DAYS, 90);
        assert_eq!(DEFAULT_LOGS_RETENTION_DAYS, 30);
        assert_eq!(DEFAULT_CLOSED_TICKETS_RETENTION_DAYS, 180);
    }

    #[test]
    fn default_cleanup_interval_in_seconds() {
        assert_eq!(DEFAULT_CLEANUP_INTERVAL_HOURS * SECS_PER_HOUR, 3600);
    }

    #[test]
    fn default_vacuum_interval_in_seconds() {
        assert_eq!(DEFAULT_VACUUM_INTERVAL_HOURS * SECS_PER_HOUR, 86400);
    }

    #[test]
    fn cleanup_config_from_worker_config() {
        // Cannot call from_env without DATABASE_URL, so test the conversion manually.
        let wc = WorkerConfig {
            database_url: "postgres://test".into(),
            api_url: "http://localhost:3000".into(),
            voice_sessions_retention_days: 60,
            logs_retention_days: 14,
            closed_tickets_retention_days: 90,
            cleanup_interval_secs: 3600,
            vacuum_enabled: true,
            vacuum_interval_secs: 86400,
        };
        let cc = CleanupConfig::from(&wc);
        assert_eq!(cc.voice_sessions_retention_days, 60);
        assert_eq!(cc.logs_retention_days, 14);
        assert_eq!(cc.closed_tickets_retention_days, 90);
    }
}
