//! Organisation — entite du jeu Influence (cf. 05.md).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::enums::influence::organization_kind::OrganizationKind;

/// Une organisation fondee par un citoyen. Le patrimoine (`treasury`) appartient
/// a l'organisation, jamais au dirigeant (05.md §9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
    pub id: Uuid,
    pub guild_id: String,
    pub kind: OrganizationKind,
    pub name: String,
    pub motto: String,
    pub treasury: i64,
    pub reputation: i64,
    pub influence: i64,
    pub founder_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub dissolved_at: Option<DateTime<Utc>>,
}
