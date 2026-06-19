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
    /// Si true, guerit la maladie (potion de soin) : statut -> healthy.
    #[allow(dead_code)]
    pub cure: bool,
}

/// Visite du compagnon d'un autre joueur (recompense le visite).
#[derive(Debug, Clone)]
pub struct VisitCommand {
    pub guild_id: String,
    pub visitor_id: String,
    pub visitor_name: String,
    pub target_id: String,
    pub xp_reward: i64,
    pub coins_reward: i64,
    pub cooldown_secs: i64,
    pub max_per_day: i64,
}

/// Resultat d'une visite (pour le message de confirmation).
#[derive(Debug, Clone)]
pub struct VisitResult {
    pub target_name: String,
    pub xp_reward: i64,
    pub coins_reward: i64,
}

/// Combat PvP asynchrone entre compagnons.
#[derive(Debug, Clone)]
pub struct CombatCommand {
    pub guild_id: String,
    pub attacker_id: String,
    pub attacker_name: String,
    pub target_id: String,
    pub energy_cost: i32,
    pub cooldown_secs: i64,
    pub elo_k: i32,
    pub xp_win: i64,
    pub xp_loss: i64,
    pub w_str: i32,
    pub w_vit: i32,
    pub w_agi: i32,
    pub random_max: i32,
}

/// Resultat d'un combat (pour le message).
#[derive(Debug, Clone)]
pub struct CombatResult {
    pub attacker_won: bool,
    pub attacker_power: i64,
    pub defender_power: i64,
    pub defender_name: String,
    pub attacker_new_elo: i32,
    pub attacker_elo_delta: i32,
}

/// Entrainement d'une stat de combat.
#[derive(Debug, Clone)]
pub struct TrainCommand {
    pub pet_id: Uuid,
    /// "str" | "vit" | "agi".
    pub stat: String,
    pub energy_cost: i32,
    pub coin_cost: i64,
    pub stat_gain: i32,
    pub cooldown_secs: i64,
}

#[async_trait]
pub trait ManagePetsUseCase: Send + Sync {
    async fn create(&self, cmd: CreatePetCommand) -> Result<Pet, DomainError>;
    async fn get_by_owner(&self, guild_id: &str, owner_id: &str) -> Result<Option<Pet>, DomainError>;
    /// Liste tous les compagnons d'une guild (vivants et morts), pour la page
    /// d'administration web (vue des dresseurs + evolution).
    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Pet>, DomainError>;
    /// Supprime definitivement un compagnon (action admin web).
    async fn delete(&self, pet_id: Uuid) -> Result<(), DomainError>;
    async fn recent_events(&self, pet_id: Uuid, limit: i64) -> Result<Vec<PetEvent>, DomainError>;
    /// Applique une action de soin (cooldown + debit coins + effet + XP).
    async fn care(&self, cmd: CareCommand) -> Result<Pet, DomainError>;

    /// Entraine une stat de combat (consomme energie + cooldown).
    async fn train(&self, cmd: TrainCommand) -> Result<Pet, DomainError>;

    /// Rend visite au compagnon d'un autre joueur (recompense le visite en
    /// XP + coins ; cooldown + limite/jour cote visiteur).
    async fn visit(&self, cmd: VisitCommand) -> Result<VisitResult, DomainError>;

    /// Combat asynchrone contre le compagnon d'un autre joueur (ELO + XP).
    async fn combat(&self, cmd: CombatCommand) -> Result<CombatResult, DomainError>;

    /// Compagnons vivants a faire decroitre (job worker, via l'API), pagine
    /// par curseur `id` croissant (`after_id = None` pour la 1re page).
    async fn list_alive(&self, limit: i64, after_id: Option<Uuid>) -> Result<Vec<Pet>, DomainError>;
    /// Applique un tick de cycle de vie (decroissance + maladie/mort) avec
    /// la config de la guild. Retourne l'evenement notable.
    async fn tick(&self, pet_id: Uuid, cfg: TickConfig) -> Result<TickOutcome, DomainError>;

    /// Enregistre la localisation de la carte Discord du joueur.
    async fn set_card_location(
        &self,
        guild_id: &str,
        owner_id: &str,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), DomainError>;
    /// Compagnons vivants ayant une carte a rafraichir (tache horaire du bot).
    async fn list_cards(&self, limit: i64, after_id: Option<Uuid>) -> Result<Vec<Pet>, DomainError>;
}
