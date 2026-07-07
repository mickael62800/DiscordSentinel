//! Use case du jeu Tamagotchi.
//!
//! La balance (couts/deltas/recompenses/cooldowns/poids de combat) est LUE et
//! CALCULEE server-side depuis la config de la guild (`bot_guild_config`,
//! composant `tamagotchi-bot`). Les commandes ne transportent plus que
//! l'action + les identifiants ; le service applique de facon atomique (debit
//! coins via le wallet partage, cooldown stocke en base, XP/niveau).

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

/// Action de soin ou d'achat boutique. Les effets/couts sont calcules
/// server-side depuis la config de la guild (cf. `balance::TamaBalance`).
#[derive(Debug, Clone)]
pub struct CareCommand {
    pub pet_id: Uuid,
    /// Action : "feed" | "play" | "sleep" | "cuddle" | "buy_<item>".
    pub action: String,
}

/// Visite du compagnon d'un autre joueur (recompense le visite).
#[derive(Debug, Clone)]
pub struct VisitCommand {
    pub guild_id: String,
    pub visitor_id: String,
    pub visitor_name: String,
    pub target_id: String,
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
}

#[async_trait]
pub trait ManagePetsUseCase: Send + Sync {
    async fn create(&self, cmd: CreatePetCommand) -> Result<Pet, DomainError>;
    async fn get_by_owner(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Option<Pet>, DomainError>;
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
    async fn list_alive(&self, limit: i64, after_id: Option<Uuid>)
        -> Result<Vec<Pet>, DomainError>;
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
    async fn list_cards(&self, limit: i64, after_id: Option<Uuid>)
        -> Result<Vec<Pet>, DomainError>;

    /// Compagnons morts dont le salon prive existe encore (reconciliation :
    /// fermer les salons orphelins d'une mort dont l'evenement a ete rate),
    /// pagine par curseur `id` croissant.
    async fn list_dead_with_channel(
        &self,
        limit: i64,
        after_id: Option<Uuid>,
    ) -> Result<Vec<Pet>, DomainError>;
    /// Efface la localisation de la carte d'un compagnon (idempotence de la
    /// reconciliation : le pet mort n'est traite qu'une fois).
    async fn clear_card_location(&self, pet_id: Uuid) -> Result<(), DomainError>;
}
