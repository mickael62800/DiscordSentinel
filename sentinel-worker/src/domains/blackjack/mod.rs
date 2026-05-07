//! Domaine blackjack : nettoyage des tables AFK (joueurs inactifs >
//! AFK_TIMEOUT_SECS).
//!
//! Porte de blackjack-cleanup-worker (Phase 1 fusion).
//!
//! Constante de timeout exposee ici pour que `cleanup_afk_tables` puisse
//! l'importer (l'ancien fichier referencait `crate::config::DEFAULT_AFK_TIMEOUT_SECS`).

pub mod cleanup_afk_tables;

/// Timeout d'inactivite avant suppression d'une table blackjack
/// (secondes). 30 min par defaut, identique a l'ancien blackjack-bot.
pub const DEFAULT_AFK_TIMEOUT_SECS: i64 = 1800;
