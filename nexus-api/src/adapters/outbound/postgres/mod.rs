//! Adapters Postgres (sqlx) implementant les ports outbound de nexus-core.

pub mod wallet_repository;
pub mod wheel_repository;

use nexus_core::domain::errors::DomainError;

/// Convertit une erreur sqlx en erreur de domaine.
pub fn pg_err(e: sqlx::Error) -> DomainError {
    DomainError::Infrastructure(e.to_string())
}
