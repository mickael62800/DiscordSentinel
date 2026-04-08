use sentinel_worker_common::{SECS_PER_HOUR, SECS_PER_MINUTE};

/// Intervalle de regeneration des points de conduite par defaut (heures).
const DEFAULT_CONDUCT_REGEN_HOURS: u64 = 1;
/// Intervalle de nettoyage des bans par defaut (minutes).
const DEFAULT_BAN_CLEANUP_MINUTES: u64 = 1;
/// Intervalle de sync des propositions de ban par defaut (minutes).
const DEFAULT_SYNC_BAN_PROPOSALS_MINUTES: u64 = 2;
/// Intervalle d'envoi des rappels par defaut (secondes).
const DEFAULT_SEND_REMINDERS_SECS: u64 = 30;

pub struct WorkerConfig {
    pub database_url: String,
    pub api_url: String,
    pub conduct_regen_interval_secs: u64,
    pub ban_cleanup_interval_secs: u64,
    pub sync_ban_proposals_interval_secs: u64,
    pub send_reminders_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        use sentinel_worker_common::{load_database_url, load_api_url, load_env};

        let regen_hours: u64 = load_env("CONDUCT_REGEN_INTERVAL", DEFAULT_CONDUCT_REGEN_HOURS);
        let cleanup_minutes: u64 = load_env("BAN_CLEANUP_INTERVAL", DEFAULT_BAN_CLEANUP_MINUTES);
        let sync_minutes: u64 = load_env("SYNC_BAN_PROPOSALS_INTERVAL", DEFAULT_SYNC_BAN_PROPOSALS_MINUTES);
        let reminders_secs: u64 = load_env("SEND_REMINDERS_INTERVAL", DEFAULT_SEND_REMINDERS_SECS);

        Self {
            database_url: load_database_url(),
            api_url: load_api_url(),
            conduct_regen_interval_secs: regen_hours * SECS_PER_HOUR,
            ban_cleanup_interval_secs: cleanup_minutes * SECS_PER_MINUTE,
            sync_ban_proposals_interval_secs: sync_minutes * SECS_PER_MINUTE,
            send_reminders_interval_secs: reminders_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_conduct_regen_is_1_hour() {
        assert_eq!(DEFAULT_CONDUCT_REGEN_HOURS * SECS_PER_HOUR, 3600);
    }

    #[test]
    fn default_ban_cleanup_is_1_minute() {
        assert_eq!(DEFAULT_BAN_CLEANUP_MINUTES * SECS_PER_MINUTE, 60);
    }

    #[test]
    fn default_sync_proposals_is_2_minutes() {
        assert_eq!(DEFAULT_SYNC_BAN_PROPOSALS_MINUTES * SECS_PER_MINUTE, 120);
    }

    #[test]
    fn default_reminders_is_30_seconds() {
        assert_eq!(DEFAULT_SEND_REMINDERS_SECS, 30);
    }
}
