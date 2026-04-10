use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{
    CoudeInsurance, CoudeInventoryItem, CoudePrime, NewCoudePrime,
};
use crate::domain::errors::DomainError;

/// Repository consolidé pour les 3 "stocks" persistants d'un joueur Coup de Coude :
/// son inventaire d'items, les primes placées sur lui et ses assurances actives.
#[async_trait]
pub trait CoudeInventoryRepository: Send + Sync {
    // ── Items ──

    async fn list_inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<CoudeInventoryItem>, DomainError>;

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

    async fn create_prime(&self, new: NewCoudePrime) -> Result<CoudePrime, DomainError>;

    async fn list_active_primes(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Vec<CoudePrime>, DomainError>;

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

    /// Crée une nouvelle assurance avec une durée fixe (1h) comptée depuis `NOW()`.
    async fn buy_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
    ) -> Result<(), DomainError>;

    async fn get_active_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoudeInsurance>, DomainError>;

    /// Désactive une assurance par ID. Retourne `false` si non trouvée.
    async fn expire_insurance(&self, insurance_id: Uuid) -> Result<bool, DomainError>;
}
