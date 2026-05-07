//! Port allocator — alloue un port libre dans un range configurable.
//!
//! Implementation Redis-backed pour la coherence cross-process (l'API + le
//! worker peuvent allouer en parallele). Utilise SETNX sur une cle par
//! port + un set "allocated" pour le free / cleanup.

use async_trait::async_trait;

use sentinel_core::domain::errors::DomainError;

/// Type de port a allouer (range different).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortKind {
    Game,
    Rcon,
}

#[async_trait]
pub trait PortAllocator: Send + Sync {
    /// Reserve un port libre dans le range configure pour ce kind.
    /// Retourne le port alloue, ou Err si plus aucun libre.
    async fn allocate(
        &self,
        kind: PortKind,
        range_start: u16,
        range_end: u16,
        owner_key: &str,
    ) -> Result<u16, DomainError>;

    /// Libere un port (a appeler au stop / delete).
    async fn release(&self, kind: PortKind, port: u16) -> Result<(), DomainError>;

    /// Verifie qu'un port est dispo (sans le reserver). Pour le reconciler.
    async fn is_available(&self, kind: PortKind, port: u16) -> Result<bool, DomainError>;
}
