use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{CombatResolution, CoudeCombat, NewCoudeCombat};
use crate::domain::errors::DomainError;

/// Use case "gérer les combats Coup de Coude".
///
/// Cycle complet d'un combat : création, transition vers paris, résolution,
/// expiration, annulation, et lectures associées.
#[async_trait]
pub trait ManageCoudeCombatsUseCase: Send + Sync {
    // ── Lecture ──

    async fn list(
        &self,
        guild_id: &str,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CoudeCombat>, DomainError>;

    async fn get(&self, id: Uuid) -> Result<CoudeCombat, DomainError>;

    async fn get_pending_for_attacker(
        &self,
        guild_id: &str,
        attacker_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError>;

    async fn get_pending_for_defender(
        &self,
        guild_id: &str,
        defender_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError>;

    async fn list_expired_pending(&self) -> Result<Vec<CoudeCombat>, DomainError>;

    /// Combat en phase de paris auquel `user_id` participe (attaquant ou défenseur).
    async fn get_betting_for_participant(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError>;

    // ── Cycle de vie ──

    async fn create(&self, new: NewCoudeCombat) -> Result<CoudeCombat, DomainError>;

    /// Annule un combat en `pending` et marque ses paris non résolus comme perdus.
    /// Erreur 404 si le combat n'existe pas ou est déjà résolu.
    async fn cancel(&self, id: Uuid) -> Result<(), DomainError>;

    /// Résout un combat actif. Erreur 409 si le combat n'est plus dans un état
    /// résoluble (race condition bot/worker).
    async fn resolve(&self, id: Uuid, resolution: CombatResolution) -> Result<(), DomainError>;

    /// Passe le combat en phase de paris. Renvoie `true` si la transition a eu
    /// lieu (combat encore en `pending`), `false` sinon.
    async fn set_betting(&self, id: Uuid, message_id: &str) -> Result<bool, DomainError>;

    async fn expire(&self, id: Uuid) -> Result<(), DomainError>;

    async fn set_defender_special(
        &self,
        id: Uuid,
        item_key: &str,
    ) -> Result<(), DomainError>;

    /// Lecture rapide du `guild_id` rattache a un combat. Renvoie `None` si le
    /// combat n'existe pas — utile pour les pre-checks RBAC `ressource-based`
    /// (RBAC ne doit PAS masquer un "combat introuvable", laisser l'erreur
    /// remonter via `cancel()` / `resolve()`).
    ///
    /// Default renvoie `NotImplemented` (mappe sur 501) pour preserver les
    /// mocks existants sans paniquer en runtime.
    async fn get_guild_id(&self, _id: Uuid) -> Result<Option<String>, DomainError> {
        Err(DomainError::NotImplemented(
            "ManageCoudeCombatsUseCase::get_guild_id".into(),
        ))
    }

    /// Purge destructive : vide toutes les tables du sous-systeme Coup de Coude
    /// pour une guild (ordre metier dans `COUDE_PURGE_TABLES`). Retourne le
    /// compte des rows supprimees par table.
    ///
    /// Admin-only, irreversible — les handlers doivent gater avec RBAC avant
    /// d'appeler.
    ///
    /// Default renvoie `NotImplemented` (mappe sur 501) pour preserver les
    /// mocks existants sans paniquer en runtime.
    async fn purge_guild_subsystem(
        &self,
        _guild_id: &str,
    ) -> Result<Vec<(String, u64)>, DomainError> {
        Err(DomainError::NotImplemented(
            "ManageCoudeCombatsUseCase::purge_guild_subsystem".into(),
        ))
    }
}
