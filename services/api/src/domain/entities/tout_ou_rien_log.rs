//! Entry persistee du Memorial des clodos (cf. COUPE_AMELIORATIONS 6.1).

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToutOuRienLogOutcome {
    Won,
    Lost,
}

impl ToutOuRienLogOutcome {
    pub fn as_db_str(self) -> &'static str {
        match self {
            ToutOuRienLogOutcome::Won => "won",
            ToutOuRienLogOutcome::Lost => "lost",
        }
    }
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "won" => Some(ToutOuRienLogOutcome::Won),
            "lost" => Some(ToutOuRienLogOutcome::Lost),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToutOuRienLogEntry {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    /// Mise = solde initial du joueur au moment du tirage.
    pub mise: i64,
    pub outcome: ToutOuRienLogOutcome,
    /// Delta wallet : positif si won (+mise), negatif si lost (-0.8*mise).
    pub delta: i64,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
#[path = "tests/tout_ou_rien_log.rs"]
mod tests;
