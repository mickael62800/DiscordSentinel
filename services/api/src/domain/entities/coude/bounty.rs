//! Primes collectives (cf. COUPE_AMELIORATIONS section 5.3 — vendetta).
//!
//! Quand un joueur atteint 5 victoires consecutives, une prime de 1000c
//! est auto-creee. Tout le monde peut contribuer via /contribuer-prime,
//! et le joueur qui bat la cible empoche tout le pot + titre Regicide.

use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

/// Seuil de declenchement de la prime automatique.
pub const BOUNTY_AUTO_OPEN_STREAK_THRESHOLD: i32 = 5;

/// Montant initial de la prime auto-creee (paye par le serveur).
pub const BOUNTY_INITIAL_AMOUNT: i64 = 1000;

/// Contribution minimum par appel /contribuer-prime.
pub const BOUNTY_MIN_CONTRIBUTION: i64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BountyStatus {
    Open,
    Claimed,
    Expired,
}

impl BountyStatus {
    pub fn as_db_str(self) -> &'static str {
        match self {
            BountyStatus::Open => "open",
            BountyStatus::Claimed => "claimed",
            BountyStatus::Expired => "expired",
        }
    }
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(BountyStatus::Open),
            "claimed" => Some(BountyStatus::Claimed),
            "expired" => Some(BountyStatus::Expired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActiveBounty {
    pub id: Uuid,
    pub guild_id: String,
    pub target_id: String,
    pub total_amount: i64,
    pub status: BountyStatus,
    pub opened_at: DateTime<Utc>,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
#[path = "tests/bounty.rs"]
mod tests;
