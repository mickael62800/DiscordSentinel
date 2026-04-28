// Bounded contexts.
pub mod ai;
pub mod audit;
pub mod casino;
pub mod community;
pub mod coude;
pub mod moderation;
pub mod system;

// Re-exports preservant l'API publique historique : tout le router fait
// `handlers::xxx::fn_name` -> chaque handler reste accessible a son ancien chemin.

// ── ai ─────────────────────────────────────────────────────────────────────
pub use ai::ai_jobs;
pub use ai::analyze;
pub use ai::analyze_image;

// ── audit ──────────────────────────────────────────────────────────────────
pub use audit::analytics;
pub use audit::audit_logs;
pub use audit::dashboard;
pub use audit::dashboard_charts;
pub use audit::discord_action_messages;
pub use audit::security;
pub use audit::stats;
pub use audit::user_activity;
pub use audit::watched_users;

// ── casino ─────────────────────────────────────────────────────────────────
pub use casino::blackjack;
pub use casino::games;
pub use casino::slot;
pub use casino::wallet;
pub use casino::wheel;

// ── community ──────────────────────────────────────────────────────────────
pub use community::conduct;
pub use community::discord_roles;
pub use community::guild_channels;
pub use community::guild_members;
pub use community::levels;
pub use community::role_panels;
pub use community::voice_channels;
pub use community::welcome;

// ── moderation (l'ancien handlers/moderation.rs est devenu moderation/actions.rs,
// glob re-export dans moderation/mod.rs preserve le path d'origine) ────────
pub use moderation::automod;
pub use moderation::infractions;
pub use moderation::notes;
pub use moderation::purge;
pub use moderation::reminders;
pub use moderation::rules;
pub use moderation::strikes;

// ── system (idem : system.rs -> info.rs, glob preserve les paths) ──────────
pub use system::bot_config;
pub use system::bot_persistence;
pub use system::cache_stats;
pub use system::exports;
pub use system::health;
pub use system::models_status;
pub use system::oauth;
pub use system::rbac;
pub use system::tickets;
