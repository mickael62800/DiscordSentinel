//! Domaine temp_roles : expiration automatique des roles temporaires
//! (worker tickle l'API qui retire les roles Discord et publie un event).
//! Porte de temp-roles-worker (Phase 2 fusion).

pub mod expire_temp_roles;
