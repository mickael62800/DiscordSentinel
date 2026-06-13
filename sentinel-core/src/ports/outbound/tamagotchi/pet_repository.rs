use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::tamagotchi::pet::{NewPet, Pet, PetEvent};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait PetRepository: Send + Sync {
    async fn create(&self, pet: NewPet) -> Result<Pet, DomainError>;
    async fn get(&self, id: Uuid) -> Result<Option<Pet>, DomainError>;
    async fn get_by_owner(&self, guild_id: &str, owner_id: &str) -> Result<Option<Pet>, DomainError>;
    /// Persiste l'etat mutable du compagnon (jauges, stats, statut, xp/level,
    /// cooldowns, timers de sante, last_decay_at).
    async fn save(&self, pet: &Pet) -> Result<Pet, DomainError>;
    /// Compagnons vivants a faire decroitre (job worker).
    async fn list_alive(&self, limit: i64) -> Result<Vec<Pet>, DomainError>;
    async fn add_event(&self, pet_id: Uuid, kind: &str, detail: &str) -> Result<(), DomainError>;
    async fn recent_events(&self, pet_id: Uuid, limit: i64) -> Result<Vec<PetEvent>, DomainError>;
}
