//! Information & enquetes (Phase 4, cf. 07.md).

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Statut d'une enquete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvestigationStatus {
    EnCours,
    Reussie,
    Echouee,
}

impl InvestigationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            InvestigationStatus::EnCours => "en_cours",
            InvestigationStatus::Reussie => "reussie",
            InvestigationStatus::Echouee => "echouee",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "en_cours" => Some(Self::EnCours),
            "reussie" => Some(Self::Reussie),
            "echouee" => Some(Self::Echouee),
            _ => None,
        }
    }
}

/// Une enquete ouverte par un citoyen sur une cible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Investigation {
    pub id: Uuid,
    pub guild_id: String,
    pub initiator_id: Uuid,
    pub initiator_user_id: String,
    pub target_user_id: String,
    pub target_username: String,
    pub subject: String,
    pub status: InvestigationStatus,
    pub resolves_at: DateTime<Utc>,
}

/// Visibilite d'une information (MVP : secret ou public).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Secret,
    Public,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Secret => "secret",
            Visibility::Public => "public",
        }
    }
    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "secret" | "prive" => Some(Self::Secret),
            "public" => Some(Self::Public),
            _ => None,
        }
    }
}

/// Une information detenue par un citoyen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Information {
    pub id: Uuid,
    pub guild_id: String,
    pub owner_id: Uuid,
    pub target_user_id: String,
    pub target_username: String,
    pub content: String,
    pub visibility: Visibility,
    pub revealed: bool,
    pub created_at: DateTime<Utc>,
}
