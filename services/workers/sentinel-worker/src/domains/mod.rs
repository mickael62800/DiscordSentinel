//! Domaines fonctionnels du worker unifie. Chaque sous-module regroupe
//! les jobs d'un domaine metier (cleanup, coude, moderation, etc.).
//! Les jobs sont autonomes : une fonction `run(deps) -> Result<...>`
//! par fichier. Aucun couplage transverse.

pub mod cleanup;
