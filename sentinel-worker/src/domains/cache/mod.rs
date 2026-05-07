//! Domaine cache : warm-up Redis pour analytics, dashboard, voice stats,
//! refresh des vues materialisees leaderboards, sync de user_cache,
//! manager des partitions Postgres futures.
//!
//! Porte de cache-worker (Phase 1 fusion). Logique inchangee, copie
//! verbatim des fichiers.

pub mod manage_partitions;
pub mod refresh_leaderboards;
pub mod sync_user_cache;
pub mod warm_analytics;
pub mod warm_dashboard;
pub mod warm_voice_stats;
