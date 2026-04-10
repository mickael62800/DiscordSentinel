/// Phase 6A — Intervalle de poll de l'API Discord audit-logs.
/// 5 minutes par defaut : compromis reactivite vs rate limit.
const DEFAULT_SYNC_INTERVAL_SECS: u64 = 300;

/// Nombre max d'entries par appel Discord (max autorise = 100).
pub const ENTRIES_PER_CALL: u32 = 100;

#[derive(Clone)]
pub struct WorkerConfig {
    pub database_url: String,
    pub api_url: String,
    pub discord_bot_token: String,
    pub sync_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        use sentinel_worker_common::{load_api_url, load_database_url, load_env};

        Self {
            database_url: load_database_url(),
            api_url: load_api_url(),
            // On tente plusieurs variantes — le meme token qu'utilise
            // moderation-bot ou automod-bot fait l'affaire.
            discord_bot_token: std::env::var("MODERATION_DISCORD_TOKEN")
                .or_else(|_| std::env::var("AUTOMOD_DISCORD_TOKEN"))
                .or_else(|_| std::env::var("DISCORD_BOT_TOKEN"))
                .unwrap_or_default(),
            sync_interval_secs: load_env("AUDIT_SYNC_INTERVAL", DEFAULT_SYNC_INTERVAL_SECS),
        }
    }

    pub fn apply_db_config(&mut self, db: &std::collections::HashMap<String, String>) {
        use sentinel_worker_common::config_or_env;
        self.sync_interval_secs = config_or_env(
            db,
            "audit_sync_interval",
            "AUDIT_SYNC_INTERVAL",
            DEFAULT_SYNC_INTERVAL_SECS,
        );
    }
}
