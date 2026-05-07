//! Domaine moderation : regeneration des points de conduit, nettoyage
//! des bans, sync des propositions de ban, envoi des rappels (DM
//! programmes via DB).
//!
//! Porte de moderation-worker (Phase 3 fusion).

pub mod cleanup_bans;
pub mod conduct_regen;
pub mod send_reminders;
pub mod sync_ban_proposals;
