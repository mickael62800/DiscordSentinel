pub mod bot_config;
pub mod bot_persistence;
pub mod cache_stats;
pub mod exports;
pub mod health;
pub mod info;
pub mod models_status;
pub mod oauth;
pub mod rbac;
pub mod tickets;

// Glob re-export du fichier `info.rs` (l'ancien `system.rs` au root)
// pour preserver `handlers::system::get_system_info` via son ancien path.
