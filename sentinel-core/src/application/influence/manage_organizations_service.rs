//! Service : gestion des organisations (creation, info, adhesion, membres).

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::influence::guild_settings::InfluenceSettings;
use crate::application::validation::{validate_non_empty, validate_positive};
use crate::domain::entities::influence::archive::RelationKind;
use crate::domain::entities::influence::treasury::TreasuryView;
use crate::domain::entities::influence::org_membership::{OrgMemberView, OrgRole};
use crate::domain::entities::influence::organization::Organization;
use crate::domain::enums::influence::organization_kind::OrganizationKind;
use crate::domain::errors::DomainError;
use crate::ports::outbound::influence::law_repository::LawRepository;
use crate::ports::inbound::influence::manage_organizations::{
    DividendResult, FundingResult, ManageOrganizationsUseCase, OrgInfo, OrgRankEntry, RolePrep,
};
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use crate::ports::outbound::influence::citizen_repository::CitizenRepository;
use crate::ports::outbound::influence::information_repository::ArchiveRepository;
use crate::ports::outbound::influence::membership_repository::MembershipRepository;
use crate::ports::outbound::influence::organization_repository::{
    NewOrganization, OrganizationRepository,
};
use crate::ports::outbound::influence::relation_repository::RelationRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

/// Longueur max d'un nom / d'une devise d'organisation.
const NAME_MAX: usize = 60;
const MOTTO_MAX: usize = 140;

pub struct ManageOrganizationsService {
    citizens: Arc<dyn CitizenRepository>,
    orgs: Arc<dyn OrganizationRepository>,
    memberships: Arc<dyn MembershipRepository>,
    relations: Option<Arc<dyn RelationRepository>>,
    archives: Option<Arc<dyn ArchiveRepository>>,
    wallet: Option<Arc<dyn WalletRepository>>,
    cfg_repo: Option<Arc<dyn BotConfigRepository>>,
    laws: Option<Arc<dyn LawRepository>>,
    rep_dims: Option<
        Arc<dyn crate::ports::outbound::influence::reputation_dims_repository::ReputationDimsRepository>,
    >,
}

impl ManageOrganizationsService {
    pub fn new(
        citizens: Arc<dyn CitizenRepository>,
        orgs: Arc<dyn OrganizationRepository>,
        memberships: Arc<dyn MembershipRepository>,
    ) -> Self {
        Self {
            citizens,
            orgs,
            memberships,
            relations: None,
            archives: None,
            wallet: None,
            cfg_repo: None,
            laws: None,
            rep_dims: None,
        }
    }

    pub fn with_wallet_repo(mut self, repo: Arc<dyn WalletRepository>) -> Self {
        self.wallet = Some(repo);
        self
    }

    pub fn with_rep_dims_repo(
        mut self,
        repo: Arc<
            dyn crate::ports::outbound::influence::reputation_dims_repository::ReputationDimsRepository,
        >,
    ) -> Self {
        self.rep_dims = Some(repo);
        self
    }

    pub fn with_law_repo(mut self, repo: Arc<dyn LawRepository>) -> Self {
        self.laws = Some(repo);
        self
    }

    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.cfg_repo = Some(repo);
        self
    }

    pub fn with_relation_repo(mut self, repo: Arc<dyn RelationRepository>) -> Self {
        self.relations = Some(repo);
        self
    }

    pub fn with_archive_repo(mut self, repo: Arc<dyn ArchiveRepository>) -> Self {
        self.archives = Some(repo);
        self
    }

    async fn settings(&self, guild_id: &str) -> InfluenceSettings {
        match &self.cfg_repo {
            Some(repo) => InfluenceSettings::load(repo.as_ref(), guild_id).await,
            None => InfluenceSettings::default(),
        }
    }

    /// Solde d'Argent (coins wallet partage) d'un citoyen.
    async fn money_balance(&self, guild_id: &str, user_id: &str) -> i64 {
        match &self.wallet {
            Some(w) => w.get(guild_id, user_id).await.ok().flatten().map(|x| x.coins).unwrap_or(0),
            None => 0,
        }
    }

    /// Resout une organisation active par nom ou renvoie `NotFound`.
    async fn require_org(&self, guild_id: &str, name: &str) -> Result<Organization, DomainError> {
        self.orgs
            .find_by_name(guild_id, name)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Aucune organisation nommée « {name} »")))
    }
}

#[async_trait]
impl ManageOrganizationsUseCase for ManageOrganizationsService {
    async fn create(
        &self,
        guild_id: &str,
        founder_user_id: &str,
        founder_username: &str,
        kind: OrganizationKind,
        name: &str,
        motto: &str,
    ) -> Result<Organization, DomainError> {
        let name = name.trim();
        let motto = motto.trim();
        validate_non_empty(name, "Le nom")?;
        if name.chars().count() > NAME_MAX {
            return Err(DomainError::ValidationError(format!(
                "Le nom ne peut pas depasser {NAME_MAX} caracteres"
            )));
        }
        if motto.chars().count() > MOTTO_MAX {
            return Err(DomainError::ValidationError(format!(
                "La devise ne peut pas depasser {MOTTO_MAX} caracteres"
            )));
        }

        let settings = self.settings(guild_id).await;
        let mut cost = settings.org_creation_cost();
        let quota = settings.org_max_per_citizen();

        let founder = self
            .citizens
            .get_or_create(guild_id, founder_user_id, founder_username, 0)
            .await?;

        // POPULARITE du fondateur : le soutien de la communaute reduit le cout
        // de creation (jusqu'a -40 %).
        if let Some(d) = &self.rep_dims {
            let pop = d.get(founder.id).await.map(|x| x.popularity).unwrap_or(0);
            cost = (cost * (100 - pop.clamp(0, 40)) / 100).max(0);
        }

        // Quota d'organisations fondees.
        if self.orgs.count_active_founded_by(founder.id).await? >= quota {
            return Err(DomainError::Forbidden(format!(
                "Tu as atteint la limite de {quota} organisations fondees."
            )));
        }

        // Argent (coins du wallet partage) suffisant ?
        let balance = self.money_balance(guild_id, founder_user_id).await;
        if balance < cost {
            return Err(DomainError::Forbidden(format!(
                "Il te faut {cost} coins pour fonder une organisation (tu en as {balance})."
            )));
        }

        // Nom deja pris ? (la contrainte UNIQUE fait aussi foi.)
        if self.orgs.find_by_name(guild_id, name).await?.is_some() {
            return Err(DomainError::Conflict(format!(
                "Une organisation nommée « {name} » existe deja."
            )));
        }

        // Debite AVANT de creer : sinon un debit qui echoue apres la creation
        // laisse une org orpheline (sans membre, nom bloque par la contrainte
        // UNIQUE). Si la creation echoue ensuite (course sur le nom), on
        // rembourse.
        match &self.wallet {
            Some(w) => {
                w.debit(guild_id, founder_user_id, cost, "influence", "Creation organisation")
                    .await?;
            }
            None => {
                self.citizens.adjust_money(founder.id, -cost).await?;
            }
        }
        let org = match self
            .orgs
            .create(NewOrganization {
                guild_id,
                kind,
                name,
                motto,
                founder_id: founder.id,
            })
            .await
        {
            Ok(o) => o,
            Err(e) => {
                // Remboursement best-effort.
                match &self.wallet {
                    Some(w) => {
                        let _ = w
                            .credit(
                                guild_id,
                                founder_user_id,
                                cost,
                                "influence",
                                "Remboursement creation org echouee",
                            )
                            .await;
                    }
                    None => {
                        let _ = self.citizens.adjust_money(founder.id, cost).await;
                    }
                }
                return Err(e);
            }
        };
        self.memberships
            .add(org.id, founder.id, OrgRole::Fondateur)
            .await?;

        // Fonder une organisation te met en avant : +NOTORIETE.
        if let Some(d) = &self.rep_dims {
            use crate::domain::entities::influence::reputation_dims::ReputationDim;
            let _ = d.adjust(founder.id, ReputationDim::Notoriety, 10).await;
        }

        // Archive best-effort : la creation d'org entre dans la memoire du serveur.
        if let Some(arch) = &self.archives {
            let _ = arch
                .append(
                    guild_id,
                    "org_created",
                    serde_json::json!({
                        "name": org.name,
                        "kind": org.kind.label(),
                        "founder": founder.username,
                    }),
                )
                .await;
        }

        Ok(org)
    }

    async fn info(&self, guild_id: &str, name: &str) -> Result<OrgInfo, DomainError> {
        let org = self.require_org(guild_id, name).await?;
        let member_count = self.memberships.count(org.id).await?;
        let relations = match &self.relations {
            Some(r) => r.list_for_org(org.id).await.unwrap_or_default(),
            None => Vec::new(),
        };
        let (collective_influence, collective_reputation) =
            self.orgs.collective_power(org.id).await.unwrap_or((0, 0));
        Ok(OrgInfo {
            org,
            member_count,
            relations,
            collective_influence,
            collective_reputation,
        })
    }

    async fn join(
        &self,
        guild_id: &str,
        name: &str,
        user_id: &str,
        username: &str,
    ) -> Result<Organization, DomainError> {
        let settings = self.settings(guild_id).await;
        let citizen = self
            .citizens
            .get_or_create(guild_id, user_id, username, settings.start_money())
            .await?;
        let org = self.require_org(guild_id, name).await?;

        if self.memberships.get(org.id, citizen.id).await?.is_some() {
            return Err(DomainError::Conflict(
                "Tu es deja membre de cette organisation.".to_string(),
            ));
        }
        self.memberships
            .add(org.id, citizen.id, OrgRole::Recrue)
            .await?;
        Ok(org)
    }

    async fn members(
        &self,
        guild_id: &str,
        name: &str,
    ) -> Result<Vec<OrgMemberView>, DomainError> {
        let org = self.require_org(guild_id, name).await?;
        self.memberships.list_views(org.id).await
    }

    async fn prepare_role(
        &self,
        guild_id: &str,
        actor_user_id: &str,
        actor_username: &str,
        is_moderator: bool,
        org_name: &str,
    ) -> Result<RolePrep, DomainError> {
        let org = self.require_org(guild_id, org_name).await?;
        if org.discord_role_id.is_some() {
            return Err(DomainError::Conflict(
                "Cette organisation a deja un role Discord.".into(),
            ));
        }
        let founder_user_id = self
            .orgs
            .founder_user_id(org.id)
            .await?
            .ok_or_else(|| DomainError::Internal("Fondateur introuvable.".into()))?;

        // Permission : fondateur (paye) ou moderateur (gratuit).
        let is_founder = actor_user_id == founder_user_id;
        if !is_founder && !is_moderator {
            return Err(DomainError::Forbidden(
                "Seuls le fondateur ou un moderateur peuvent creer le role.".into(),
            ));
        }

        // Le fondateur paie en coins (sauf s'il agit en moderateur). On VALIDE
        // seulement le solde ici ; le DEBIT reel est fait dans set_role, une fois
        // le role Discord cree — sinon un echec/relance de creation re-debiterait
        // a chaque tentative sans jamais poser de role.
        if is_founder && !is_moderator {
            let cost = self.settings(guild_id).await.org_role_cost();
            let balance = self.money_balance(guild_id, actor_user_id).await;
            if balance < cost {
                return Err(DomainError::Forbidden(format!(
                    "Le role coute {cost} coins (tu en as {balance})."
                )));
            }
        }
        let _ = actor_username;

        Ok(RolePrep {
            founder_user_id,
            org_name: org.name,
        })
    }

    async fn set_role(
        &self,
        guild_id: &str,
        org_name: &str,
        role_id: &str,
        actor_user_id: &str,
        is_moderator: bool,
    ) -> Result<(), DomainError> {
        let org = self.require_org(guild_id, org_name).await?;
        self.orgs.set_discord_role(org.id, role_id).await?;

        // Debit du fondateur (gratuit pour un modo), APRES que le role existe :
        // c'est le seul point de paiement (best-effort ; un echec de debit ne
        // detruit pas le role deja cree).
        let founder_user_id = self.orgs.founder_user_id(org.id).await?.unwrap_or_default();
        if actor_user_id == founder_user_id && !is_moderator {
            let cost = self.settings(guild_id).await.org_role_cost();
            if cost > 0 {
                if let Some(w) = &self.wallet {
                    let _ = w
                        .debit(guild_id, actor_user_id, cost, "influence", "Role d'organisation")
                        .await;
                }
            }
        }
        Ok(())
    }

    async fn set_channel(
        &self,
        guild_id: &str,
        org_name: &str,
        channel_id: &str,
    ) -> Result<(), DomainError> {
        let org = self.require_org(guild_id, org_name).await?;
        self.orgs.set_discord_channel(org.id, channel_id).await
    }

    async fn set_relation(
        &self,
        guild_id: &str,
        actor_user_id: &str,
        actor_username: &str,
        org_name: &str,
        other_org_name: &str,
        relation: RelationKind,
    ) -> Result<(), DomainError> {
        let Some(relations) = &self.relations else {
            return Err(DomainError::NotImplemented(
                "Relations indisponibles.".into(),
            ));
        };
        let org = self.require_org(guild_id, org_name).await?;
        let other = self.require_org(guild_id, other_org_name).await?;
        if org.id == other.id {
            return Err(DomainError::ValidationError(
                "Une organisation ne peut pas se lier a elle-meme.".into(),
            ));
        }

        // L'acteur doit etre dirigeant (ou fondateur) de l'organisation source.
        let settings = self.settings(guild_id).await;
        let actor = self
            .citizens
            .get_or_create(guild_id, actor_user_id, actor_username, settings.start_money())
            .await?;
        let membership = self
            .memberships
            .get(org.id, actor.id)
            .await?
            .ok_or_else(|| {
                DomainError::Forbidden("Tu n'es pas membre de cette organisation.".into())
            })?;
        if membership.role.rank() > OrgRole::Dirigeant.rank() {
            return Err(DomainError::Forbidden(
                "Seuls le fondateur et les dirigeants peuvent declarer une relation.".into(),
            ));
        }

        relations.set(guild_id, org.id, other.id, relation).await?;

        if let Some(arch) = &self.archives {
            let _ = arch
                .append(
                    guild_id,
                    "org_relation",
                    serde_json::json!({
                        "org": org.name,
                        "other": other.name,
                        "relation": relation.label(),
                    }),
                )
                .await;
        }
        Ok(())
    }

    async fn ranking(&self, guild_id: &str) -> Result<Vec<OrgRankEntry>, DomainError> {
        let mut orgs = self.orgs.list_for_guild(guild_id).await?;
        orgs.sort_by_key(|o| std::cmp::Reverse(o.treasury));
        orgs.truncate(15);
        let mut out = Vec::with_capacity(orgs.len());
        for o in orgs {
            let member_count = self.memberships.count(o.id).await.unwrap_or(0);
            let (collective_influence, _) = self.orgs.collective_power(o.id).await.unwrap_or((0, 0));
            out.push(OrgRankEntry {
                name: o.name,
                treasury: o.treasury,
                member_count,
                collective_influence,
            });
        }
        Ok(out)
    }

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
    ) -> Result<FundingResult, DomainError> {
        validate_positive(amount, "Le financement")?;
        let Some(laws) = &self.laws else {
            return Err(DomainError::Internal("Lois indisponibles.".into()));
        };
        let law_uuid = uuid::Uuid::parse_str(law_id)
            .map_err(|_| DomainError::ValidationError("Identifiant de loi invalide.".into()))?;
        let law = laws
            .get(law_uuid)
            .await?
            .ok_or_else(|| DomainError::NotFound("Loi introuvable.".into()))?;
        let org = self.require_org(guild_id, org_name).await?;
        let settings = self.settings(guild_id).await;
        let actor = self
            .citizens
            .get_or_create(guild_id, actor_user_id, actor_username, settings.start_money())
            .await?;
        let can = self
            .memberships
            .get(org.id, actor.id)
            .await?
            .map(|m| m.role.can_manage_treasury())
            .unwrap_or(false);
        if !can {
            return Err(DomainError::Forbidden(
                "Seuls le fondateur et les dirigeants peuvent financer une loi.".into(),
            ));
        }
        // Depense la tresorerie (puits : les coins sont consommes en lobbying,
        // credites a personne). Retrait garde.
        if self
            .orgs
            .withdraw_treasury(org.id, guild_id, amount, actor_user_id, actor_username)
            .await?
            .is_none()
        {
            return Err(DomainError::Forbidden(
                "Tresorerie insuffisante pour ce financement.".into(),
            ));
        }
        let (pour_delta, contre_delta) = if camp_pour { (amount, 0) } else { (0, amount) };
        // Ajoute le poids ; si la loi n'est plus en vote, rembourse la tresorerie.
        if !laws.add_funding(law_uuid, pour_delta, contre_delta).await? {
            let _ = self
                .orgs
                .deposit_treasury(org.id, guild_id, amount, actor_user_id, actor_username)
                .await;
            return Err(DomainError::Conflict("Le vote de cette loi est clos.".into()));
        }
        let updated = laws.get(law_uuid).await?;
        let (funding_pour, funding_contre) = updated
            .map(|l| (l.funding_pour, l.funding_contre))
            .unwrap_or((pour_delta, contre_delta));
        let treasury_left = self.orgs.get(org.id).await?.map(|o| o.treasury).unwrap_or(0);
        Ok(FundingResult {
            law_title: law.title,
            amount,
            camp_pour,
            funding_pour,
            funding_contre,
            treasury_left,
        })
    }

    async fn distribute_dividend(
        &self,
        guild_id: &str,
        org_name: &str,
        actor_user_id: &str,
        actor_username: &str,
        per_member: i64,
    ) -> Result<DividendResult, DomainError> {
        validate_positive(per_member, "Le dividende")?;
        let org = self.require_org(guild_id, org_name).await?;
        let settings = self.settings(guild_id).await;
        let actor = self
            .citizens
            .get_or_create(guild_id, actor_user_id, actor_username, settings.start_money())
            .await?;
        let can = self
            .memberships
            .get(org.id, actor.id)
            .await?
            .map(|m| m.role.can_manage_treasury())
            .unwrap_or(false);
        if !can {
            return Err(DomainError::Forbidden(
                "Seuls le fondateur et les dirigeants peuvent verser des dividendes.".into(),
            ));
        }
        let Some(w) = &self.wallet else {
            return Err(DomainError::Internal("Wallet indisponible.".into()));
        };
        let members = self.memberships.list_member_user_ids(org.id).await?;
        // Chaque versement est atomique (retrait garde + credit) ; on s'arrete
        // des que la tresorerie ne couvre plus un membre.
        let mut paid = 0i64;
        for (uid, _uname) in &members {
            let bal = self
                .orgs
                .withdraw_treasury(org.id, guild_id, per_member, actor_user_id, actor_username)
                .await?;
            if bal.is_none() {
                break;
            }
            if w
                .credit(guild_id, uid, per_member, "influence-treasury", "Dividende d'organisation")
                .await
                .is_err()
            {
                let _ = self
                    .orgs
                    .deposit_treasury(org.id, guild_id, per_member, actor_user_id, actor_username)
                    .await;
                break;
            }
            paid += 1;
        }
        let treasury_left = self.orgs.get(org.id).await?.map(|o| o.treasury).unwrap_or(0);
        Ok(DividendResult {
            paid_count: paid,
            per_member,
            total: paid * per_member,
            treasury_left,
        })
    }

    async fn treasury(&self, guild_id: &str, org_name: &str) -> Result<TreasuryView, DomainError> {
        let org = self.require_org(guild_id, org_name).await?;
        let movements = self.orgs.list_treasury_movements(org.id, 10).await?;
        Ok(TreasuryView {
            org_name: org.name,
            balance: org.treasury,
            movements,
        })
    }

    async fn deposit_treasury(
        &self,
        guild_id: &str,
        org_name: &str,
        actor_user_id: &str,
        actor_username: &str,
        amount: i64,
    ) -> Result<TreasuryView, DomainError> {
        validate_positive(amount, "Le don")?;
        let org = self.require_org(guild_id, org_name).await?;
        let settings = self.settings(guild_id).await;
        let citizen = self
            .citizens
            .get_or_create(guild_id, actor_user_id, actor_username, settings.start_money())
            .await?;
        // Tout membre peut alimenter la tresorerie.
        if self.memberships.get(org.id, citizen.id).await?.is_none() {
            return Err(DomainError::Forbidden(
                "Tu dois etre membre de l'organisation pour l'alimenter.".into(),
            ));
        }
        let Some(w) = &self.wallet else {
            return Err(DomainError::Internal("Wallet indisponible.".into()));
        };
        // Debite le membre (echoue si solde insuffisant), puis credite la
        // tresorerie ; rembourse le membre si l'ecriture tresorerie echoue.
        w.debit(guild_id, actor_user_id, amount, "influence-treasury", "Don a l'organisation")
            .await?;
        if let Err(e) = self
            .orgs
            .deposit_treasury(org.id, guild_id, amount, actor_user_id, actor_username)
            .await
        {
            let _ = w
                .credit(guild_id, actor_user_id, amount, "influence-treasury", "Remboursement don echoue")
                .await;
            return Err(e);
        }
        // Donner publiquement a son org renforce la POPULARITE (proportionnel au
        // don, plafonne pour eviter le farm de micro-dons).
        if let Some(d) = &self.rep_dims {
            use crate::domain::entities::influence::reputation_dims::ReputationDim;
            let gain = (amount / 100).clamp(1, 25);
            let _ = d.adjust(citizen.id, ReputationDim::Popularity, gain).await;
        }
        self.treasury(guild_id, org_name).await
    }

    async fn withdraw_treasury(
        &self,
        guild_id: &str,
        org_name: &str,
        actor_user_id: &str,
        actor_username: &str,
        amount: i64,
    ) -> Result<TreasuryView, DomainError> {
        validate_positive(amount, "Le retrait")?;
        let org = self.require_org(guild_id, org_name).await?;
        let settings = self.settings(guild_id).await;
        let citizen = self
            .citizens
            .get_or_create(guild_id, actor_user_id, actor_username, settings.start_money())
            .await?;
        // Seuls Dirigeant+ peuvent retirer/depenser.
        let member = self.memberships.get(org.id, citizen.id).await?;
        let can = member.map(|m| m.role.can_manage_treasury()).unwrap_or(false);
        if !can {
            return Err(DomainError::Forbidden(
                "Seuls le fondateur et les dirigeants peuvent retirer de la tresorerie.".into(),
            ));
        }
        // Retrait GARDE cote tresorerie, puis credit du wallet ; re-depot si le
        // credit echoue.
        let new_bal = self
            .orgs
            .withdraw_treasury(org.id, guild_id, amount, actor_user_id, actor_username)
            .await?;
        if new_bal.is_none() {
            return Err(DomainError::Forbidden(
                "Tresorerie insuffisante pour ce retrait.".into(),
            ));
        }
        let Some(w) = &self.wallet else {
            return Err(DomainError::Internal("Wallet indisponible.".into()));
        };
        if let Err(e) = w
            .credit(guild_id, actor_user_id, amount, "influence-treasury", "Retrait de tresorerie")
            .await
        {
            let _ = self
                .orgs
                .deposit_treasury(org.id, guild_id, amount, actor_user_id, actor_username)
                .await;
            return Err(e);
        }
        self.treasury(guild_id, org_name).await
    }

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
    ) -> Result<TreasuryView, DomainError> {
        validate_positive(amount, "Le paiement")?;
        let org = self.require_org(guild_id, org_name).await?;
        let settings = self.settings(guild_id).await;
        // Acteur : Dirigeant+.
        let actor = self
            .citizens
            .get_or_create(guild_id, actor_user_id, actor_username, settings.start_money())
            .await?;
        let can = self
            .memberships
            .get(org.id, actor.id)
            .await?
            .map(|m| m.role.can_manage_treasury())
            .unwrap_or(false);
        if !can {
            return Err(DomainError::Forbidden(
                "Seuls le fondateur et les dirigeants peuvent payer un membre.".into(),
            ));
        }
        // Beneficiaire : doit etre membre de l'org.
        let benef = self
            .citizens
            .get_or_create(guild_id, beneficiary_user_id, beneficiary_username, settings.start_money())
            .await?;
        if self.memberships.get(org.id, benef.id).await?.is_none() {
            return Err(DomainError::Forbidden(
                "Le beneficiaire doit etre membre de l'organisation.".into(),
            ));
        }
        let Some(w) = &self.wallet else {
            return Err(DomainError::Internal("Wallet indisponible.".into()));
        };
        // Retrait GARDE de la tresorerie, puis credit du BENEFICIAIRE (re-depot
        // si le credit echoue).
        let new_bal = self
            .orgs
            .withdraw_treasury(org.id, guild_id, amount, actor_user_id, actor_username)
            .await?;
        if new_bal.is_none() {
            return Err(DomainError::Forbidden(
                "Tresorerie insuffisante pour ce paiement.".into(),
            ));
        }
        if let Err(e) = w
            .credit(guild_id, beneficiary_user_id, amount, "influence-treasury", "Salaire d'organisation")
            .await
        {
            let _ = self
                .orgs
                .deposit_treasury(org.id, guild_id, amount, actor_user_id, actor_username)
                .await;
            return Err(e);
        }
        self.treasury(guild_id, org_name).await
    }
}
