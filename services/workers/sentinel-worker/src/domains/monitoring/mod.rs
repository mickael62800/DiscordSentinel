//! Domaine monitoring : surveillance presence des bots/workers via
//! les cles Redis `bot:online:{name}`. Detecte transitions online/offline
//! et alerte via /api/logs + event Redis.
//!
//! Porte de monitoring-worker (Phase 1B fusion). Structure differente
//! des autres jobs car la boucle maintient un etat (`previous_online`)
//! entre les iterations -> ne rentre pas dans `spawn_periodic` qui
//! ne porte pas d'etat. On garde donc une fonction `start()` qui spawn
//! sa propre task.

pub mod check_services;

#[derive(Clone)]
pub struct MonitorConfig {
    pub api_url: String,
    pub api_key: String,
    pub check_interval_secs: u64,
}
