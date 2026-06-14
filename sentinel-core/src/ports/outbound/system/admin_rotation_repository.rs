use async_trait::async_trait;

use crate::domain::entities::system::admin_rotation::{RotationState, ServedEntry};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait AdminRotationRepository: Send + Sync {
    async fn get(&self, guild_id: &str) -> Result<Option<RotationState>, DomainError>;
    /// Cree ou met a jour l'etat complet de la rotation.
    async fn upsert(&self, state: &RotationState) -> Result<(), DomainError>;
    /// Enregistre qu'un user a ete admin (now).
    async fn record_served(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    /// Derniere date de mandat par user (1 entree par user), pour le
    /// round-robin (jamais servi / plus ancien d'abord).
    async fn served_entries(&self, guild_id: &str) -> Result<Vec<ServedEntry>, DomainError>;
}
