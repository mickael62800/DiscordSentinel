//! Use case : gestion des organisations (creation, info, adhesion, membres).

use async_trait::async_trait;

use crate::domain::entities::influence::archive::{OrgRelation, RelationKind};
use crate::domain::entities::influence::org_membership::OrgMemberView;
use crate::domain::entities::influence::organization::Organization;
use crate::domain::entities::influence::treasury::TreasuryView;
use crate::domain::enums::influence::organization_kind::OrganizationKind;
use crate::domain::errors::DomainError;

/// Autorisation de creation de role : a qui attribuer + nom de l'orga.
#[derive(Debug, Clone)]
pub struct RolePrep {
    pub founder_user_id: String,
    pub org_name: String,
}

/// Vue d'ensemble d'une organisation (`/org info`).
#[derive(Debug, Clone)]
pub struct OrgInfo {
    pub org: Organization,
    pub member_count: i64,
    pub relations: Vec<OrgRelation>,
    /// Puissance collective = somme des capitaux des membres.
    pub collective_influence: i64,
    pub collective_reputation: i64,
}

/// Une ligne du palmares des organisations (`/org classement`).
#[derive(Debug, Clone)]
pub struct OrgRankEntry {
    pub name: String,
    pub treasury: i64,
    pub member_count: i64,
    pub collective_influence: i64,
}

/// Resultat d'une distribution de dividendes (`/org dividende`).
#[derive(Debug, Clone)]
pub struct DividendResult {
    pub paid_count: i64,
    pub per_member: i64,
    pub total: i64,
    pub treasury_left: i64,
}

/// Resultat du financement d'une loi par une org (`/loi financer`).
#[derive(Debug, Clone)]
pub struct FundingResult {
    pub law_title: String,
    pub amount: i64,
    pub camp_pour: bool,
    pub funding_pour: i64,
    pub funding_contre: i64,
    pub treasury_left: i64,
}

#[async_trait]
pub trait ManageOrganizationsUseCase: Send + Sync {
    /// Fonde une organisation : verifie le quota et le cout (debite l'Argent du
    /// fondateur), cree l'org et l'adhesion Fondateur.
    async fn create(
        &self,
        guild_id: &str,
        founder_user_id: &str,
        founder_username: &str,
        kind: OrganizationKind,
        name: &str,
        motto: &str,
    ) -> Result<Organization, DomainError>;

    /// Informations sur une organisation par nom.
    async fn info(&self, guild_id: &str, name: &str) -> Result<OrgInfo, DomainError>;

    /// Dissout une organisation (soft-delete). Reservee au FONDATEUR. Renvoie
    /// l'org dissoute (pour que l'appelant supprime ses salons Discord).
    async fn dissolve(
        &self,
        guild_id: &str,
        name: &str,
        actor_user_id: &str,
    ) -> Result<Organization, DomainError>;

    /// Rejoint une organisation comme Recrue.
    async fn join(
        &self,
        guild_id: &str,
        name: &str,
        user_id: &str,
        username: &str,
    ) -> Result<Organization, DomainError>;

    /// Liste des membres d'une organisation (pour affichage).
    async fn members(
        &self,
        guild_id: &str,
        name: &str,
    ) -> Result<Vec<OrgMemberView>, DomainError>;

    /// Autorise la creation du role Discord d'une orga : verifie que l'acteur
    /// est le fondateur (paie le cout en coins) ou un moderateur (gratuit), et
    /// renvoie le discord user_id du fondateur (a qui attribuer le role).
    async fn prepare_role(
        &self,
        guild_id: &str,
        actor_user_id: &str,
        actor_username: &str,
        is_moderator: bool,
        org_name: &str,
    ) -> Result<RolePrep, DomainError>;

    /// Lie le role Discord cree a l'organisation ET debite le fondateur (seul
    /// point de paiement, apres creation effective du role).
    async fn set_role(
        &self,
        guild_id: &str,
        org_name: &str,
        role_id: &str,
        actor_user_id: &str,
        is_moderator: bool,
    ) -> Result<(), DomainError>;

    /// Lie le salon prive Discord auto-cree a la fondation a l'organisation.
    async fn set_channel(
        &self,
        guild_id: &str,
        org_name: &str,
        channel_id: &str,
    ) -> Result<(), DomainError>;

    /// Declare une relation d'une organisation vers une autre. L'acteur doit
    /// etre dirigeant (ou fondateur) de l'organisation source.
    async fn set_relation(
        &self,
        guild_id: &str,
        actor_user_id: &str,
        actor_username: &str,
        org_name: &str,
        other_org_name: &str,
        relation: RelationKind,
    ) -> Result<(), DomainError>;

    /// Palmares des organisations du serveur, triees par tresor de guerre.
    async fn ranking(&self, guild_id: &str) -> Result<Vec<OrgRankEntry>, DomainError>;

    /// Finance une loi depuis la tresorerie d'une org (Dirigeant+) : depense des
    /// coins (puits) pour ajouter du poids a un camp. `camp_pour` = pour/contre.
    #[allow(clippy::too_many_arguments)]
    async fn fund_law(
        &self,
        guild_id: &str,
        org_name: &str,
        law_id: &str,
        actor_user_id: &str,
        actor_username: &str,
        amount: i64,
        camp_pour: bool,
    ) -> Result<FundingResult, DomainError>;

    /// Verse `per_member` coins a CHAQUE membre depuis la tresorerie (Dirigeant+).
    /// S'arrete quand la tresorerie est epuisee ; renvoie le nombre paye.
    async fn distribute_dividend(
        &self,
        guild_id: &str,
        org_name: &str,
        actor_user_id: &str,
        actor_username: &str,
        per_member: i64,
    ) -> Result<DividendResult, DomainError>;

    /// Consulte la tresorerie d'une organisation (solde + derniers mouvements).
    async fn treasury(&self, guild_id: &str, org_name: &str) -> Result<TreasuryView, DomainError>;

    /// Reverse des coins du wallet du membre vers la tresorerie de l'org
    /// (tout membre). Debite le wallet, incremente la tresorerie.
    async fn deposit_treasury(
        &self,
        guild_id: &str,
        org_name: &str,
        actor_user_id: &str,
        actor_username: &str,
        amount: i64,
    ) -> Result<TreasuryView, DomainError>;

    /// Retire des coins de la tresorerie vers le wallet de l'acteur (Dirigeant+).
    async fn withdraw_treasury(
        &self,
        guild_id: &str,
        org_name: &str,
        actor_user_id: &str,
        actor_username: &str,
        amount: i64,
    ) -> Result<TreasuryView, DomainError>;

    /// Paie un MEMBRE depuis la tresorerie (salaire/prime) : -tresorerie -> wallet
    /// du beneficiaire. Reserve aux Dirigeant+ ; le beneficiaire doit etre membre.
    #[allow(clippy::too_many_arguments)]
    async fn pay_member(
        &self,
        guild_id: &str,
        org_name: &str,
        actor_user_id: &str,
        actor_username: &str,
        beneficiary_user_id: &str,
        beneficiary_username: &str,
        amount: i64,
    ) -> Result<TreasuryView, DomainError>;
}
