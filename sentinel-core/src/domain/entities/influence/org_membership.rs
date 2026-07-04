//! Adhesion et hierarchie d'une organisation (cf. 05.md §5).

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Role hierarchique d'un membre, du plus haut (Fondateur) au plus bas (Recrue).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrgRole {
    Fondateur,
    Dirigeant,
    Responsable,
    Membre,
    Recrue,
}

impl OrgRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrgRole::Fondateur => "fondateur",
            OrgRole::Dirigeant => "dirigeant",
            OrgRole::Responsable => "responsable",
            OrgRole::Membre => "membre",
            OrgRole::Recrue => "recrue",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "fondateur" => Some(Self::Fondateur),
            "dirigeant" => Some(Self::Dirigeant),
            "responsable" => Some(Self::Responsable),
            "membre" => Some(Self::Membre),
            "recrue" => Some(Self::Recrue),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            OrgRole::Fondateur => "Fondateur",
            OrgRole::Dirigeant => "Dirigeant",
            OrgRole::Responsable => "Responsable",
            OrgRole::Membre => "Membre",
            OrgRole::Recrue => "Recrue",
        }
    }

    /// Rang hierarchique : 0 = Fondateur (plus haut). Sert aux comparaisons de
    /// pouvoir (recruter/exclure).
    pub fn rank(&self) -> u8 {
        match self {
            OrgRole::Fondateur => 0,
            OrgRole::Dirigeant => 1,
            OrgRole::Responsable => 2,
            OrgRole::Membre => 3,
            OrgRole::Recrue => 4,
        }
    }

    /// Peut recruter de nouveaux membres (Responsable et au-dessus).
    pub fn can_recruit(&self) -> bool {
        self.rank() <= OrgRole::Responsable.rank()
    }
}

/// Un membre d'une organisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgMember {
    pub id: Uuid,
    pub org_id: Uuid,
    pub citizen_id: Uuid,
    pub role: OrgRole,
    pub joined_at: DateTime<Utc>,
}

/// Vue enrichie pour l'affichage (`/org membres`) : role + pseudo du citoyen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgMemberView {
    pub username: String,
    pub role: OrgRole,
    pub joined_at: DateTime<Utc>,
}
