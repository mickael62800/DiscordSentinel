//! Domaines fonctionnels du worker unifie. Chaque sous-module regroupe
//! les jobs d'un domaine metier (cleanup, cache, ai, etc.).
//! Les jobs sont autonomes : une fonction `run(deps) -> Result<...>`
//! par fichier. Aucun couplage transverse.

pub mod ai;
pub mod analytics;
pub mod announcements;
pub mod appeal_sla;
pub mod audit_cache;
pub mod automod;
pub mod cache;
pub mod cleanup;
pub mod discord_audit_sync;
pub mod export;
pub mod guild_backup;
pub mod moderation;
pub mod monitoring;
pub mod security;
pub mod temp_roles;
pub mod tickets;
