use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

/// Combat 1v1 du mini-jeu Coup de Coude.
///
/// Cycle de vie : `pending` → `accepted` → `betting` → `resolved`/`expired`/`refused`.
#[derive(Debug, Clone)]
pub struct CoudeCombat {
    pub id: Uuid,
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub status: String,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>,
    pub special_attack: Option<String>,
    pub defender_special: Option<String>,
    pub coins_transferred: Option<i64>,
    pub result_message: Option<String>,
    pub message_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl CoudeCombat {
    /// Statuts qui correspondent à un combat encore "vivant" (résoluble).
    pub const ACTIVE_STATUSES: &'static [&'static str] = &["pending", "accepted", "betting"];

    pub fn is_active(&self) -> bool {
        Self::ACTIVE_STATUSES.contains(&self.status.as_str())
    }
}

/// Données nécessaires pour créer un nouveau combat.
#[derive(Debug, Clone)]
pub struct NewCoudeCombat {
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub special_attack: Option<String>,
}

/// Données pour clôturer un combat (succès ou expiration via worker).
#[derive(Debug, Clone)]
pub struct CombatResolution {
    pub status: String,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>,
    pub result_message: Option<String>,
    pub coins_transferred: i64,
}

#[cfg(test)]
#[path = "tests/combat.rs"]
mod tests;
