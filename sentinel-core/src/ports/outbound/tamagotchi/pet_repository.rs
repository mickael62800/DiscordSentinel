use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::tamagotchi::pet::{NewPet, Pet, PetEvent};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait PetRepository: Send + Sync {
    async fn create(&self, pet: NewPet) -> Result<Pet, DomainError>;
    async fn get(&self, id: Uuid) -> Result<Option<Pet>, DomainError>;
    async fn get_by_owner(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Option<Pet>, DomainError>;
    /// Liste tous les compagnons d'une guild (tous statuts), pour l'admin web.
    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Pet>, DomainError>;
    /// Supprime definitivement un compagnon (et ses events via cascade).
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    /// Persiste l'etat mutable du compagnon (jauges, stats, statut, xp/level,
    /// cooldowns, timers de sante, last_decay_at).
    async fn save(&self, pet: &Pet) -> Result<Pet, DomainError>;
    /// Compagnons vivants a faire decroitre (job worker), pagine par curseur
    /// `id` croissant. `after_id = None` pour la premiere page ; passer le
    /// dernier id recu pour la page suivante. Tri par `id` (stable malgre les
    /// morts et les mutations de `last_decay_at` pendant le tick).
    async fn list_alive(&self, limit: i64, after_id: Option<Uuid>)
        -> Result<Vec<Pet>, DomainError>;
    /// Enregistre la localisation (channel + message) de la carte Discord du
    /// joueur, pour le rafraichissement automatique par le bot.
    async fn set_card_location(
        &self,
        guild_id: &str,
        owner_id: &str,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), DomainError>;
    /// Compagnons vivants ayant une carte postee (a rafraichir), pagine par
    /// curseur `id` croissant.
    async fn list_with_card(
        &self,
        limit: i64,
        after_id: Option<Uuid>,
    ) -> Result<Vec<Pet>, DomainError>;
    async fn add_event(&self, pet_id: Uuid, kind: &str, detail: &str) -> Result<(), DomainError>;
    async fn recent_events(&self, pet_id: Uuid, limit: i64) -> Result<Vec<PetEvent>, DomainError>;
}
