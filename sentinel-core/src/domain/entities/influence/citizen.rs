//! Le citoyen — entite racine du jeu Influence (cf. 03.md).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::influence::capital::Capitals;

/// Un citoyen d'un serveur : identite + ses 5 capitaux.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citizen {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub capitals: Capitals,
    pub joined_at: DateTime<Utc>,
}
