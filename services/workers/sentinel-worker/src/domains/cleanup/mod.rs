//! Domaine cleanup : retention/purge des donnees historiques + VACUUM.
//! Porte du worker `cleanup-worker` (Phase 1 de la fusion).

pub mod cleanup_old_data;
pub mod vacuum_tables;
