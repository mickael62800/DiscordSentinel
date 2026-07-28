//! Domaine temp_roles : expiration automatique des roles temporaires
//! (worker tickle l'API qui retire les roles Discord et publie un event).
//! Porte de temp-roles-worker (Phase 2 fusion).

//! SQL assumé : scan ensembliste des lignes `temp_roles` échues (échéance =
//! timestamp posé en DB à l'attribution) + XADD ; le bot exécute le retrait
//! Discord et DELETE la ligne. Aucune décision métier en Rust côté worker.

pub mod expire_temp_roles;
