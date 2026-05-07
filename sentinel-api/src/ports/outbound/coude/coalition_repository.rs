//! Port outbound pour les coalitions (cf. COUPE_AMELIORATIONS 5.3).

use async_trait::async_trait;
use uuid::Uuid;

use sentinel_core::domain::entities::coude::coalition::ActiveCoalition;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait CoalitionRepository: Send + Sync {
    /// Cree une coalition + ajoute le 1er membre. Echec Conflict si une
    /// coalition active/forming existe deja.
    async fn create_with_first_member(
        &self,
        guild_id: &str,
        target_id: &str,
        first_member_id: &str,
        first_member_name: &str,
        duration_hours: i64,
    ) -> Result<Uuid, DomainError>;

    /// Ajoute un membre a une coalition existante. No-op si deja membre.
    /// Si l ajout fait passer le compte a >= MIN_MEMBERS, transite
    /// status='forming' -> 'active'. Retourne la coalition mise a jour.
    async fn add_member(
        &self,
        coalition_id: Uuid,
        member_id: &str,
        member_name: &str,
    ) -> Result<ActiveCoalition, DomainError>;

    /// Recupere la coalition active ou forming sur une cible (None si
    /// aucune).
    async fn get_active(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveCoalition>, DomainError>;

    /// Marque la coalition comme cassee par `breaker_id`.
    async fn mark_broken(
        &self,
        coalition_id: Uuid,
        breaker_id: &str,
    ) -> Result<(), DomainError>;

    /// Retourne `true` si user_id est membre d une coalition active
    /// contre target_id.
    async fn is_member_of_active_coalition_against(
        &self,
        guild_id: &str,
        member_id: &str,
        target_id: &str,
    ) -> Result<bool, DomainError>;
}
