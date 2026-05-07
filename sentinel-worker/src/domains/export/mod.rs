//! Domaine export : depile la file `export_jobs` et genere les fichiers
//! demandes (CSV/JSON). Le scan est tres rapide (5s par defaut) car le
//! client web attend sa piece jointe.
//!
//! Porte de export-worker (Phase 2 fusion). Les constantes de garde-fou
//! restent ici parce que `drain_export_jobs.rs` les importe.

pub mod drain_export_jobs;

/// Timeout au-dela duquel un job 'processing' est considere zombie et
/// reset a 'pending' pour retry (protection contre crash worker en
/// plein job).
pub const PROCESSING_TIMEOUT_SECS: i64 = 300;

/// Nombre max de lignes par export (garde-fou memoire — 50k lignes en
/// JSON font 20-50 MB selon la richesse, au-dela passer par un storage
/// externe est plus sage).
pub const MAX_ROWS_PER_EXPORT: i64 = 50_000;
