//! Domaines fonctionnels du worker unifie. Chaque sous-module regroupe
//! les jobs d'un domaine metier (cleanup, cache, audit_cache, etc.).
//! Les jobs sont autonomes : une fonction `run(deps) -> Result<...>`
//! par fichier. Aucun couplage transverse.

pub mod audit_cache;
pub mod blackjack;
pub mod cache;
pub mod cleanup;
pub mod monitoring;
