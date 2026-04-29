//! Entry persistee du Memorial des clodos (cf. COUPE_AMELIORATIONS 6.1).

use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;
use crate::domain::entities::system::discord_ids::UserId;

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
    pub user_id: UserId,
    pub username: String,
    /// Mise = solde initial du joueur au moment du tirage.
    pub mise: i64,
    pub outcome: ToutOuRienLogOutcome,
    /// Delta wallet : positif si won (+mise), negatif si lost (-0.8*mise).
    pub delta: i64,
    pub created_at: DateTime<Utc>,
}

/// Stats agregees d un joueur sur ses tentatives /tout-ou-rien.
/// Utilise dans /profil pour afficher l historique personnel.
#[derive(Debug, Clone, Default)]
pub struct ToutOuRienUserStats {
    pub attempts: i64,
    pub wins: i64,
    pub losses: i64,
    /// Plus gros gain (delta positif max). 0 si jamais gagne.
    pub biggest_win: i64,
    /// Plus grosse perte (delta negatif min, en valeur absolue). 0 si
    /// jamais perdu.
    pub biggest_loss: i64,
}

impl ToutOuRienUserStats {
    pub fn never_played(&self) -> bool {
        self.attempts == 0
    }
}

#[cfg(test)]
#[path = "tests/tout_ou_rien_log.rs"]
mod tests;
