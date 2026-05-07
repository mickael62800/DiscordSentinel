//! Session joueur : trace une connexion/deconnexion sur un serveur.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSession {
    pub id: Uuid,
    pub server_id: Uuid,
    pub player_name: String,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
}

impl PlayerSession {
    pub fn is_active(&self) -> bool {
        self.left_at.is_none()
    }
}
