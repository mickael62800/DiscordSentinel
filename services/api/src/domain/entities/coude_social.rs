use chrono::{DateTime, Utc};
use uuid::Uuid;

// ══════════════════════════════════════════════════════════════════════
// ── Leaderboard ──
// ══════════════════════════════════════════════════════════════════════

/// Catégorie de classement supportée par `/api/coude/{guild}/leaderboard/{category}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderboardCategory {
    Richest,
    Thieves,
    Cowards,
    Chaos,
    Level,
}

impl LeaderboardCategory {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "richest" => Some(Self::Richest),
            "thieves" => Some(Self::Thieves),
            "cowards" => Some(Self::Cowards),
            "chaos" => Some(Self::Chaos),
            "level" => Some(Self::Level),
            _ => None,
        }
    }
}

/// Entrée d'un classement. `value` = critère du classement (coins, level, etc.)
#[derive(Debug, Clone)]
pub struct CoudeLeaderboardEntry {
    pub user_id: String,
    pub username: String,
    pub value: i64,
}

// ══════════════════════════════════════════════════════════════════════
// ── Événements serveur ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct CoudeEvent {
    pub id: Uuid,
    pub guild_id: String,
    pub event_type: String,
    pub active: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ══════════════════════════════════════════════════════════════════════
// ── Daily chaos ──
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct NewDailyChaos {
    pub guild_id: String,
    pub loser_id: String,
    pub loser_name: String,
    pub winner_id: String,
    pub winner_name: String,
    pub amount: i64,
}

/// Resultat d'un trigger de chaos journalier reussi, pret a etre affiche.
#[derive(Debug, Clone)]
pub struct DailyChaosOutcome {
    pub loser_id: String,
    pub loser_name: String,
    pub winner_id: String,
    pub winner_name: String,
    pub amount: i64,
    pub channel_id: String,
}

// ══════════════════════════════════════════════════════════════════════
// ── Saison ──
// ══════════════════════════════════════════════════════════════════════

/// État d'une saison telle qu'exposée au bot (numéro, fenêtre temporelle,
/// jours restants). Durée standard : 90 jours depuis `started_at`.
#[derive(Debug, Clone)]
pub struct CoudeCurrentSeason {
    pub season_number: i32,
    pub started_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub days_remaining: i64,
}
