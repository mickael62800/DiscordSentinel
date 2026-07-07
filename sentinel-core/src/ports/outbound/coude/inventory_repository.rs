use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::inventory::Insurance;
use crate::domain::entities::coude::inventory::InventoryItem;
use crate::domain::entities::coude::inventory::NewCoudePrime;
use crate::domain::entities::coude::inventory::Prime;
use crate::domain::errors::DomainError;

/// Resultat de la consommation atomique d'une potion (item + heal en une tx).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsePotionTxOutcome {
    /// Item consomme et HP mis a jour.
    Healed {
        actually_healed: i32,
        new_hp: i32,
        hp_max: i32,
    },
    /// Le joueur n'avait pas la potion (aucune quantite consommee).
    NoItem,
    /// Le joueur etait deja a pleine sante (aucun item consomme).
    AlreadyFull,
}

/// Repository consolidé pour les 3 "stocks" persistants d'un joueur Coup de Coude :
/// son inventaire d'items, les primes placées sur lui et ses assurances actives.
#[async_trait]
pub trait InventoryRepository: Send + Sync {
    // ── Items ──

    /// Consomme une potion et applique le heal (clamp au HP max) dans UNE
    /// transaction atomique : decremente l'item ET met a jour les HP du
    /// joueur, ou rien du tout. Le montant nominal `heal_amount` est fourni
    /// par le service (bareme domain). Default `unimplemented!()` pour que
    /// les mocks qui ne l'appellent pas continuent de compiler.
    async fn use_potion_atomic(
        &self,
        _guild_id: &str,
        _user_id: &str,
        _item_key: &str,
        _heal_amount: i32,
    ) -> Result<UsePotionTxOutcome, DomainError> {
        unimplemented!("use_potion_atomic not implemented")
    }

    async fn list_inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<InventoryItem>, DomainError>;

    /// Upsert : +1 à la quantité existante, ou crée la ligne avec quantity=1.
    async fn add_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<(), DomainError>;

    /// Décrémente la quantité. Retourne `false` si le joueur n'avait pas l'item.
    async fn use_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, DomainError>;

    async fn has_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, DomainError>;

    // ── Primes ──

    async fn create_prime(&self, new: NewCoudePrime) -> Result<Prime, DomainError>;

    async fn list_active_primes(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Vec<Prime>, DomainError>;

    /// Marque toutes les primes non claimées sur `target_id` comme claimées par
    /// `claimer_id` et retourne le montant total encaissé.
    async fn claim_primes(
        &self,
        guild_id: &str,
        target_id: &str,
        claimer_id: &str,
        claimer_name: &str,
    ) -> Result<i64, DomainError>;

    // ── Assurances ──

    /// Crée une nouvelle assurance avec une durée en secondes comptée
    /// depuis `NOW()`. `duration_seconds <= 0` → fallback 3600s (1h).
    /// Retourne `true` si la ligne a ete inseree, `false` si une assurance
    /// active existait deja (race-safe via INSERT...WHERE NOT EXISTS).
    async fn buy_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        duration_seconds: i64,
    ) -> Result<bool, DomainError>;

    /// Variante avec nombre max d assurances actives concurrentes (cf.
    /// COUPE_AMELIORATIONS 3.2, palier niveau 5 : 2 emplacements au lieu
    /// de 1). Default impl delegue a `buy_insurance` (= max=1).
    async fn buy_insurance_with_max_slots(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        duration_seconds: i64,
        _max_slots: i32,
    ) -> Result<bool, DomainError> {
        self.buy_insurance(guild_id, user_id, is_scam, duration_seconds)
            .await
    }

    async fn get_active_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Insurance>, DomainError>;

    /// Désactive une assurance par ID. Retourne `false` si non trouvée.
    async fn expire_insurance(&self, insurance_id: Uuid) -> Result<bool, DomainError>;
}
