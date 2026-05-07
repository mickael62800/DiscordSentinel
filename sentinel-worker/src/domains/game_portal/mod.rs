//! Domaine game_portal : 4 jobs HTTP-triggered (health-check,
//! idle-shutdown, reconcile, image-cleanup) qui appellent l'API a
//! intervalles fixes.
//!
//! Porte de game-portal-worker (Phase 2 fusion).

pub mod jobs;
