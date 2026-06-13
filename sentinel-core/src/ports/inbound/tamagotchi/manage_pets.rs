//! Use case du jeu Tamagotchi.
//!
//! Le bot lit la config guild (couts/gains/cooldowns) et les passe dans les
//! commandes ; l'API applique de facon atomique (debit coins via le wallet
//! partage, cooldown stocke en base, XP/niveau, transitions de sante).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::tamagotchi::pet::{Pet, PetEvent, TickConfig, TickOutcome};
use crate::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct CreatePetCommand {
    pub guild_id: String,
    pub owner_id: String,
    pub name: String,
    /// Espece choisie (sanglier, renard, ...).
    pub species: String,
}

/// Effets/couts d'une action de soin (issus de la config guild).
#[derive(Debug, Clone)]
pub struct CareCommand {
    pub pet_id: Uuid,
    /// Action : "feed" | "play" | "sleep" | "cuddle".
    pub action: String,
    /// Cout en coins (debite du wallet partage). 0 = gratuit.
    pub coin_cost: i64,
    /// Variations de jauges (peuvent etre negatives).
    pub hunger_delta: i32,
    pub happiness_delta: i32,
    pub energy_delta: i32,
    /// XP gagne.
    pub xp_gain: i64,
    /// Cooldown a appliquer (secondes).
    pub cooldown_secs: i64,
}

#[async_trait]
pub trait ManagePetsUseCase: Send + Sync {
    async fn create(&self, cmd: CreatePetCommand) -> Result<Pet, DomainError>;
    async fn get_by_owner(&self, guild_id: &str, owner_id: &str) -> Result<Option<Pet>, DomainError>;
    async fn recent_events(&self, pet_id: Uuid, limit: i64) -> Result<Vec<PetEvent>, DomainError>;
    /// Applique une action de soin (cooldown + debit coins + effet + XP).
    async fn care(&self, cmd: CareCommand) -> Result<Pet, DomainError>;

    /// Compagnons vivants a faire decroitre (job worker, via l'API).
    async fn list_alive(&self, limit: i64) -> Result<Vec<Pet>, DomainError>;
    /// Applique un tick de cycle de vie (decroissance + maladie/mort) avec
    /// la config de la guild. Retourne l'evenement notable.
    async fn tick(&self, pet_id: Uuid, cfg: TickConfig) -> Result<TickOutcome, DomainError>;
}
