use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{
    CoudeInsurance, CoudeInventoryItem, CoudePrime, NewCoudePrime,
};
use crate::domain::errors::DomainError;

/// Use case "gérer l'inventaire/primes/assurances Coup de Coude".
#[async_trait]
pub trait ManageCoudeInventoryUseCase: Send + Sync {
    // ── Items ──
    async fn list_inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<CoudeInventoryItem>, DomainError>;

    async fn add_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<(), DomainError>;

    /// Retourne `true` si un item a effectivement été consommé.
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

    async fn claim_primes(
        &self,
        guild_id: &str,
        target_id: &str,
        claimer_id: &str,
        claimer_name: &str,
    ) -> Result<i64, DomainError>;

    // ── Assurances ──
    async fn buy_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        duration_seconds: i64,
    ) -> Result<(), DomainError>;

    async fn get_active_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoudeInsurance>, DomainError>;

    async fn expire_insurance(&self, insurance_id: Uuid) -> Result<(), DomainError>;
}
