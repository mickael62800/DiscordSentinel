//! Mouvement de capital — entree du registre append-only (Phase 2).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::influence::capital::Capital;

/// Une variation de capital tracee (credit ou debit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapitalMovement {
    pub id: Uuid,
    pub capital: Capital,
    pub delta: i64,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}
