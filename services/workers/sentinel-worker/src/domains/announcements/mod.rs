//! Domaine announcements : publication automatique des annonces
//! planifiees, alignee sur HH:00:00 UTC. A chaque tick, fetch des
//! annonces dues via API puis XADD sur stream Redis pour le bot.
//!
//! Porte de announcement-worker (Phase 2 fusion). Structure custom
//! (tick aligne sur l'heure pile + Redis streams), pas un simple
//! spawn_periodic.

pub mod publish_due;
