//! Domaine audit_cache : refresh periodique du cache `watched_users`
//! pour audit-bot. Pousse en Redis + emet `watched_users_refreshed` sur
//! la stream sentinel:events.
//!
//! Porte de audit-cache-worker (Phase 1 fusion).

pub mod refresh_watched_users;
