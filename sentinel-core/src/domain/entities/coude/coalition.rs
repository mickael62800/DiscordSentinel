//! Coalitions (cf. COUPE_AMELIORATIONS 5.3 — vendetta extras).
//!
//! 3+ joueurs se liguent contre une cible. Chacun paye 500c. La cible
//! subit -20% sur tous ses gains pendant 48h, OU jusqu a ce qu elle
//! batte UN des conspirateurs en combat direct.

use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;
use crate::domain::entities::system::discord_ids::GuildId;

pub const COALITION_COST_PER_MEMBER: i64 = 500;
pub const COALITION_MIN_MEMBERS: i32 = 3;
pub const COALITION_DURATION_HOURS: i64 = 48;
/// Multiplicateur sur les gains (vol, paris, casino, combats) de la
/// cible tant que la coalition est active.
pub const COALITION_GAIN_MULTIPLIER: f64 = 0.80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoalitionStatus {
    /// Coalition ouverte mais < COALITION_MIN_MEMBERS — pas encore active.
    Forming,
    /// >= 3 membres et dans la fenetre 48h — penalite appliquee.
    Active,
    /// Cassee par la cible (combat gagne contre un membre).
    Broken,
    /// Fenetre 48h ecoulee sans cassage.
    Expired,
}

impl CoalitionStatus {
    pub fn as_db_str(self) -> &'static str {
        match self {
            CoalitionStatus::Forming => "forming",
            CoalitionStatus::Active => "active",
            CoalitionStatus::Broken => "broken",
            CoalitionStatus::Expired => "expired",
        }
    }
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "forming" => Some(CoalitionStatus::Forming),
            "active" => Some(CoalitionStatus::Active),
            "broken" => Some(CoalitionStatus::Broken),
            "expired" => Some(CoalitionStatus::Expired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActiveCoalition {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub target_id: String,
    pub opened_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: CoalitionStatus,
    pub broken_by: Option<String>,
    pub broken_at: Option<DateTime<Utc>>,
    pub members: Vec<CoalitionMember>,
}

#[derive(Debug, Clone)]
pub struct CoalitionMember {
    pub member_id: String,
    pub member_name: String,
    pub joined_at: DateTime<Utc>,
}

impl ActiveCoalition {
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.status == CoalitionStatus::Active && self.expires_at > now
    }

    /// Doit-on faire transitionner forming -> active ?
    pub fn should_become_active(&self) -> bool {
        self.status == CoalitionStatus::Forming
            && (self.members.len() as i32) >= COALITION_MIN_MEMBERS
    }
}

/// Applique le multiplicateur coalition a un gain.
pub fn apply_coalition_penalty(gain: i64, has_active_coalition: bool) -> i64 {
    if !has_active_coalition || gain <= 0 {
        return gain;
    }
    (gain as f64 * COALITION_GAIN_MULTIPLIER) as i64
}

#[cfg(test)]
#[path = "tests/coalition.rs"]
mod tests;
