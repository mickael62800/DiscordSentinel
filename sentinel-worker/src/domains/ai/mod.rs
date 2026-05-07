//! Domaine ai : depile la file `ai_jobs` et dispatch vers l'inference
//! cote API. Poll tres rapide (2s par defaut) — les bots attendent une
//! analyse rapide.
//!
//! Porte de ai-worker (Phase 2 fusion).

pub mod drain_ai_jobs;
