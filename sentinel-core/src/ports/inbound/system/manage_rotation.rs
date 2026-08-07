//! Use case de l'administrateur tournant. Persistance de l'etat + historique.
//! L'orchestration Discord (MP, choix du candidat, roles) est faite par le
//! bot ; ici on ne gere que l'etat persistant.

use async_trait::async_trait;

use crate::domain::entities::system::admin_rotation::{RotationState, ServedEntry};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ManageRotationUseCase: Send + Sync {
    /// Etat courant (idle par defaut si absent).
    async fn get_state(&self, guild_id: &str) -> Result<RotationState, DomainError>;
    async fn save_state(&self, state: RotationState) -> Result<(), DomainError>;
    async fn record_served(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn served_entries(&self, guild_id: &str) -> Result<Vec<ServedEntry>, DomainError>;
}
