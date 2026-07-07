//! Tentative de vol persistee (`coude_steal_attempts`, Phase 5).
//!
//! Le bot Discord persiste chaque /voler ici ; le worker `expire_steals`
//! scanne les pending expires. Ces structs decrivent une tentative a inserer
//! et le resultat renvoye au bot (id + fin de fenetre de defense).

use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

/// Nouvelle tentative de vol prete a etre persistee (statut `pending`).
#[derive(Debug, Clone)]
pub struct NewStealAttempt {
    pub id: Uuid,
    pub guild_id: String,
    pub thief_id: String,
    pub target_id: String,
    pub message_id: String,
    pub channel_id: String,
    pub expires_at: DateTime<Utc>,
}

/// Resultat renvoye au bot apres creation : identifiant persistant et instant
/// de fermeture de la fenetre de defense.
#[derive(Debug, Clone)]
pub struct CreatedStealAttempt {
    pub id: Uuid,
    pub expires_at: DateTime<Utc>,
}
