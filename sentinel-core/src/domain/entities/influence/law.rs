//! Loi — objet du cycle legislatif (depot -> vote -> application). Phase 3.
//!
//! Pour le MVP : une loi proposee ouvre immediatement un vote binaire de tous
//! les citoyens, cloture a l'echeance par le worker (adoptee si pour>contre).

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Statut du cycle de vie d'une loi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LawStatus {
    /// En cours de vote.
    Vote,
    Adoptee,
    Rejetee,
}

impl LawStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LawStatus::Vote => "vote",
            LawStatus::Adoptee => "adoptee",
            LawStatus::Rejetee => "rejetee",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "vote" | "depot" | "debat" => Some(Self::Vote),
            "adoptee" => Some(Self::Adoptee),
            "rejetee" => Some(Self::Rejetee),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            LawStatus::Vote => "En vote",
            LawStatus::Adoptee => "Adoptée",
            LawStatus::Rejetee => "Rejetée",
        }
    }
}

/// Une loi soumise au vote des citoyens du serveur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Law {
    pub id: Uuid,
    pub guild_id: String,
    pub title: String,
    pub body: String,
    pub status: LawStatus,
    pub author_id: Uuid,
    pub closes_at: Option<DateTime<Utc>>,
    pub channel_id: Option<String>,
    pub message_id: Option<String>,
}
