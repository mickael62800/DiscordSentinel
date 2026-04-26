//! Ultimates par classe (cf. COUPE_AMELIORATIONS 3.1).
//!
//! Mirror cote API du catalogue declare cote bot. Les 4 kinds sont :
//! - bourrin : HP swap pre-combat
//! - agile   : coin flip pur 50/50
//! - fourbe  : vol de la mise pre-combat
//! - tank    : statue (no damage / forfait)
//!
//! Logique pure ici : enum + cooldown helpers. La persistance vit dans
//! `coude_ultimate_states`.

use chrono::{DateTime, Duration, Utc};

/// Niveau minimum pour utiliser une ultimate.
pub const ULTIMATE_UNLOCK_LEVEL: i32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UltimateKind {
    Bourrin,
    Agile,
    Fourbe,
    Tank,
}

impl UltimateKind {
    pub fn as_db_str(self) -> &'static str {
        match self {
            UltimateKind::Bourrin => "bourrin",
            UltimateKind::Agile => "agile",
            UltimateKind::Fourbe => "fourbe",
            UltimateKind::Tank => "tank",
        }
    }
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "bourrin" => Some(UltimateKind::Bourrin),
            "agile" => Some(UltimateKind::Agile),
            "fourbe" => Some(UltimateKind::Fourbe),
            "tank" => Some(UltimateKind::Tank),
            _ => None,
        }
    }

    /// Cooldown en jours entre 2 utilisations. Fourbe = 14 jours, autres = 7.
    pub fn cooldown_days(self) -> i64 {
        match self {
            UltimateKind::Fourbe => 14,
            _ => 7,
        }
    }

    /// Class_key (table coude_players.class) qui a acces a cet ultimate.
    pub fn class_key(self) -> &'static str {
        match self {
            UltimateKind::Bourrin => "bourrin",
            UltimateKind::Agile => "agile",
            UltimateKind::Fourbe => "fourbe",
            UltimateKind::Tank => "tank",
        }
    }
}

/// Verifie si un joueur peut activer son ultimate :
/// - level >= ULTIMATE_UNLOCK_LEVEL
/// - cooldown ecoule depuis last_used_at
pub fn ultimate_ready(level: i32, kind: UltimateKind, last_used_at: Option<DateTime<Utc>>) -> bool {
    if level < ULTIMATE_UNLOCK_LEVEL {
        return false;
    }
    let Some(last) = last_used_at else {
        return true;
    };
    let cooldown = Duration::days(kind.cooldown_days());
    Utc::now() >= last + cooldown
}

#[derive(Debug, Clone)]
pub struct UltimateState {
    pub guild_id: String,
    pub user_id: String,
    pub pending_kind: Option<UltimateKind>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub activated_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
#[path = "tests/ultimate.rs"]
mod tests;
