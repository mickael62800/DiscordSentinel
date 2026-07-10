//! Port outbound : persistance des organisations.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::influence::organization::Organization;
use crate::domain::entities::influence::treasury::TreasuryMovement;
use crate::domain::enums::influence::organization_kind::OrganizationKind;
use crate::domain::errors::DomainError;

/// Parametres de creation d'une organisation.
pub struct NewOrganization<'a> {
    pub guild_id: &'a str,
    pub kind: OrganizationKind,
    pub name: &'a str,
    pub motto: &'a str,
    pub founder_id: Uuid,
}

#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    /// Cree une organisation. Renvoie `Conflict` si le nom existe deja.
    async fn create(&self, new: NewOrganization<'_>) -> Result<Organization, DomainError>;

    /// Recupere une organisation par id (active ou dissoute).
    async fn get(&self, id: Uuid) -> Result<Option<Organization>, DomainError>;

    /// Recupere une organisation active par nom (insensible a la casse).
    async fn find_by_name(
        &self,
        guild_id: &str,
        name: &str,
    ) -> Result<Option<Organization>, DomainError>;

    /// Nombre d'organisations actives fondees par ce citoyen.
    async fn count_active_founded_by(&self, founder_id: Uuid) -> Result<i64, DomainError>;

    /// Liste des organisations actives du serveur.
    async fn list_for_guild(&self, guild_id: &str) -> Result<Vec<Organization>, DomainError>;

    /// Lie un role Discord a une organisation.
    async fn set_discord_role(&self, org_id: Uuid, role_id: &str) -> Result<(), DomainError>;

    /// Lie le salon prive Discord auto-cree a une organisation.
    async fn set_discord_channel(&self, org_id: Uuid, channel_id: &str)
        -> Result<(), DomainError>;

    /// Discord user_id du fondateur d'une organisation (JOIN citizens).
    async fn founder_user_id(&self, org_id: Uuid) -> Result<Option<String>, DomainError>;

    /// Puissance COLLECTIVE d'une org : (influence, reputation) sommees sur les
    /// capitaux de ses membres. Reflete son poids politique reel.
    async fn collective_power(&self, org_id: Uuid) -> Result<(i64, i64), DomainError>;

    /// Incremente la tresorerie (depot) + enregistre le mouvement. Renvoie le
    /// nouveau solde.
    async fn deposit_treasury(
        &self,
        org_id: Uuid,
        guild_id: &str,
        amount: i64,
        actor_user_id: &str,
        actor_username: &str,
    ) -> Result<i64, DomainError>;

    /// Decremente la tresorerie (retrait) de facon GARDEE (`treasury >= amount`)
    /// + enregistre le mouvement. `None` si solde insuffisant. Renvoie le
    /// nouveau solde.
    async fn withdraw_treasury(
        &self,
        org_id: Uuid,
        guild_id: &str,
        amount: i64,
        actor_user_id: &str,
        actor_username: &str,
    ) -> Result<Option<i64>, DomainError>;

    /// Derniers mouvements de tresorerie (plus recents d'abord).
    async fn list_treasury_movements(
        &self,
        org_id: Uuid,
        limit: i64,
    ) -> Result<Vec<TreasuryMovement>, DomainError>;
}
