//! Motion — sujet d'un vote binaire au sein d'une organisation.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Statut du cycle de vie d'une motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionStatus {
    Ouverte,
    Adoptee,
    Rejetee,
}

impl MotionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MotionStatus::Ouverte => "ouverte",
            MotionStatus::Adoptee => "adoptee",
            MotionStatus::Rejetee => "rejetee",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "ouverte" => Some(Self::Ouverte),
            "adoptee" => Some(Self::Adoptee),
            "rejetee" => Some(Self::Rejetee),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            MotionStatus::Ouverte => "Ouverte",
            MotionStatus::Adoptee => "Adoptée",
            MotionStatus::Rejetee => "Rejetée",
        }
    }
}

/// Une motion soumise au vote des membres d'une organisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Motion {
    pub id: Uuid,
    pub guild_id: String,
    pub org_id: Uuid,
    pub title: String,
    pub status: MotionStatus,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub closes_at: Option<DateTime<Utc>>,
}
