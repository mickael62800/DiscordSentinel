// Service bloque dans sentinel-api : manage_stats_service tient un redis::Client.
pub mod manage_stats_service;

// Re-export des services purs depuis sentinel-core.
pub use sentinel_core::application::audit::{
    manage_audit_logs_service, manage_discord_action_messages_service,
    manage_security_service, manage_watched_users_service,
};
