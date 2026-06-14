//! Etat de la rotation de l'administrateur tournant (cf. migration 259).

use chrono::{DateTime, Utc};

/// Etat persiste de la machine de rotation pour une guild.
#[derive(Debug, Clone)]
pub struct RotationState {
    pub guild_id: String,
    /// idle | offering_candidate | awaiting_owner | offering_stay
    pub state: String,
    pub current_admin_id: Option<String>,
    pub current_admin_since: Option<DateTime<Utc>>,
    pub period_start: Option<DateTime<Utc>>,
    pub next_rotation_at: Option<DateTime<Utc>>,
    pub candidate_id: Option<String>,
    pub candidate_offered_at: Option<DateTime<Utc>>,
    /// IDs deja sollicites durant la rotation en cours.
    pub asked_this_round: Vec<String>,
}

impl RotationState {
    /// Etat initial (idle) pour une guild qui n'a pas encore de ligne.
    pub fn idle(guild_id: &str) -> Self {
        Self {
            guild_id: guild_id.to_string(),
            state: "idle".to_string(),
            current_admin_id: None,
            current_admin_since: None,
            period_start: None,
            next_rotation_at: None,
            candidate_id: None,
            candidate_offered_at: None,
            asked_this_round: Vec::new(),
        }
    }
}

/// Une entree d'historique (qui a ete admin, et quand).
#[derive(Debug, Clone)]
pub struct ServedEntry {
    pub user_id: String,
    pub served_at: DateTime<Utc>,
}
