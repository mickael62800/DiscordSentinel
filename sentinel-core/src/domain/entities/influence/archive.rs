//! Archives & relations inter-organisations (Phase 5).

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Une entree de la memoire du serveur (03.md §12 / 07.md §13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub event_type: String,
    pub payload: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}

/// Nature d'une relation entre deux organisations (05.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Alliance,
    Rivalite,
    Boycott,
}

impl RelationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationKind::Alliance => "alliance",
            RelationKind::Rivalite => "rivalite",
            RelationKind::Boycott => "boycott",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "alliance" => Some(Self::Alliance),
            "rivalite" => Some(Self::Rivalite),
            "boycott" => Some(Self::Boycott),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            RelationKind::Alliance => "Alliance",
            RelationKind::Rivalite => "Rivalité",
            RelationKind::Boycott => "Boycott",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            RelationKind::Alliance => "🤝",
            RelationKind::Rivalite => "⚔️",
            RelationKind::Boycott => "🚫",
        }
    }
}

/// Une relation dirigee d'une organisation vers une autre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgRelation {
    pub id: Uuid,
    pub other_org_name: String,
    pub relation: RelationKind,
}
