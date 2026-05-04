//! Domaine discord_audit_sync : poll periodique de l'API Discord
//! audit-logs et persistance en base. Intervalle 5 min par defaut
//! (compromis reactivite vs rate limit Discord).
//!
//! Porte de discord-audit-sync-worker (Phase 2 fusion).

pub mod sync_discord_audit_logs;

/// Nombre max d'entries par appel Discord (max autorise = 100).
pub const ENTRIES_PER_CALL: u32 = 100;
