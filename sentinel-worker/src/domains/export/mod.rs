//! Domaine export : depile la file `export_jobs` et genere les fichiers
//! demandes (CSV/JSON). Le scan est tres rapide (5s par defaut) car le
//! client web attend sa piece jointe.
//!
//! Porte de export-worker (Phase 2 fusion). Les garde-fous (timeout zombie,
//! cap lignes) viennent desormais de `WorkerConfig` (const/env/DB) et sont
//! passes a `drain_export_jobs::run`.

pub mod drain_export_jobs;
