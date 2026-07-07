use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::inventory::Insurance;
use crate::domain::entities::coude::inventory::InventoryItem;
use crate::domain::entities::coude::inventory::NewCoudePrime;
use crate::domain::entities::coude::inventory::Prime;
use crate::domain::errors::DomainError;

/// Resultat de l'usage d'une potion de soin hors combat (bareme + heal
/// resolus server-side). Le bot n'a plus qu'a rendre le message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsePotionResult {
    /// L'item n'est pas une potion utilisable.
    NotAPotion,
    /// Le joueur est deja a pleine sante.
    AlreadyFull,
    /// Gaspillage : la potion soigne bien plus que le manque de HP.
    Wasteful { hp_missing: i32, heal_amount: i32 },
    /// Le joueur n'a pas la potion en inventaire.
    NoItem,
    /// Potion consommee + heal applique (clamp au HP max).
    Healed {
        actually_healed: i32,
        new_hp: i32,
        hp_max: i32,
    },
}

/// Use case "gérer l'inventaire/primes/assurances Coup de Coude".
#[async_trait]
pub trait ManageCoudeInventoryUseCase: Send + Sync {
    /// Utilise une potion de soin hors combat. Resout le bareme (montant de
    /// heal), applique la regle anti-gaspillage et le clamp au HP max, puis
    /// consomme l'item + met a jour les HP dans UNE transaction atomique.
    /// Default `unimplemented!()` pour ne pas casser les mocks existants.
    async fn use_potion(
        &self,
        _guild_id: &str,
        _user_id: &str,
        _item_key: &str,
    ) -> Result<UsePotionResult, DomainError> {
        unimplemented!("use_potion not implemented")
    }

    // ── Items ──
    async fn list_inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<InventoryItem>, DomainError>;

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
    async fn create_prime(&self, new: NewCoudePrime) -> Result<Prime, DomainError>;

    async fn list_active_primes(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Vec<Prime>, DomainError>;

    async fn claim_primes(
        &self,
        guild_id: &str,
        target_id: &str,
        claimer_id: &str,
        claimer_name: &str,
    ) -> Result<i64, DomainError>;

    // ── Assurances ──
    /// Retourne `true` si l'assurance a ete creee, `false` si une assurance
    /// active existait deja (dans quel cas l'appelant doit rembourser).
    async fn buy_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        duration_seconds: i64,
    ) -> Result<bool, DomainError>;

    /// Variante de `buy_insurance` avec niveau du joueur passe explicitement
    /// (cf. COUPE_AMELIORATIONS 3.2 palier niveau 5 : 2 slots actives au
    /// lieu de 1). Default impl delegue a `buy_insurance` (= 1 slot).
    async fn buy_insurance_for_level(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
        duration_seconds: i64,
        _level: i32,
    ) -> Result<bool, DomainError> {
        self.buy_insurance(guild_id, user_id, is_scam, duration_seconds)
            .await
    }

    /// Phase 2 #3 audit : decide cote API si l'assurance est un scam
    /// (`gen_range(1..=100) <= scam_rate_pct`) et persiste avec le verdict.
    /// Le bot ne fait plus de RNG.
    ///
    /// Retourne `(created, is_scam)`. `created == false` => assurance active
    /// existait deja, le caller doit rembourser.
    /// Default impl Ok((false, false)) pour preserver les mocks existants.
    async fn buy_insurance_with_scam_roll(
        &self,
        _guild_id: &str,
        _user_id: &str,
        _scam_rate_pct: u32,
        _duration_seconds: i64,
        _level: i32,
    ) -> Result<(bool, bool), DomainError> {
        Err(DomainError::NotImplemented(
            "ManageCoudeInventoryUseCase::buy_insurance_with_scam_roll".into(),
        ))
    }

    async fn get_active_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Insurance>, DomainError>;

    async fn expire_insurance(&self, insurance_id: Uuid) -> Result<(), DomainError>;
}
