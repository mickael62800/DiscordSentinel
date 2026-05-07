use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::inventory::Insurance;
use crate::domain::entities::coude::inventory::InventoryItem;
use crate::domain::entities::coude::inventory::Prime;
use crate::domain::entities::coude::inventory::NewCoudePrime;
use crate::domain::errors::DomainError;

/// Repository consolidé pour les 3 "stocks" persistants d'un joueur Coup de Coude :
/// son inventaire d'items, les primes placées sur lui et ses assurances actives.
#[async_trait]
pub trait InventoryRepository: Send + Sync {
    // ── Items ──

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
        self.buy_insurance(guild_id, user_id, is_scam, duration_seconds).await
    }

    async fn get_active_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Insurance>, DomainError>;

    /// Désactive une assurance par ID. Retourne `false` si non trouvée.
    async fn expire_insurance(&self, insurance_id: Uuid) -> Result<bool, DomainError>;
}
