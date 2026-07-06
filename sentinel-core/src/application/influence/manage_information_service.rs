//! Service : information & medias (enquetes, intel, revelation).

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use rand::Rng;
use uuid::Uuid;

use crate::application::influence::guild_settings::InfluenceSettings;
use crate::application::validation::validate_non_empty;
use crate::domain::entities::influence::capital::Capital;
use crate::domain::entities::influence::information::{
    Information, Investigation, InvestigationStatus, Visibility,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::influence::manage_information::{
    ManageInformationUseCase, ResolvedInvestigation, RevealOutcome,
};
use crate::ports::outbound::influence::citizen_repository::CitizenRepository;
use crate::ports::outbound::influence::information_repository::{
    ArchiveRepository, InformationRepository, InvestigationRepository, NewInformation,
    NewInvestigation,
};
use crate::ports::outbound::influence::movement_repository::MovementRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

const SUBJECT_MAX: usize = 200;

pub struct ManageInformationService {
    citizens: Arc<dyn CitizenRepository>,
    investigations: Arc<dyn InvestigationRepository>,
    information: Arc<dyn InformationRepository>,
    archives: Arc<dyn ArchiveRepository>,
    movements: Arc<dyn MovementRepository>,
    wallet: Option<Arc<dyn crate::ports::outbound::casino::wallet_repository::WalletRepository>>,
    cfg_repo: Option<Arc<dyn BotConfigRepository>>,
    rep_dims: Option<
        Arc<dyn crate::ports::outbound::influence::reputation_dims_repository::ReputationDimsRepository>,
    >,
}

impl ManageInformationService {
    pub fn new(
        citizens: Arc<dyn CitizenRepository>,
        investigations: Arc<dyn InvestigationRepository>,
        information: Arc<dyn InformationRepository>,
        archives: Arc<dyn ArchiveRepository>,
        movements: Arc<dyn MovementRepository>,
    ) -> Self {
        Self {
            citizens,
            investigations,
            information,
            archives,
            movements,
            wallet: None,
            cfg_repo: None,
            rep_dims: None,
        }
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

    pub fn with_wallet_repo(
        mut self,
        repo: Arc<dyn crate::ports::outbound::casino::wallet_repository::WalletRepository>,
    ) -> Self {
        self.wallet = Some(repo);
        self
    }

    async fn money_balance(&self, guild_id: &str, user_id: &str) -> i64 {
        match &self.wallet {
            Some(w) => w.get(guild_id, user_id).await.ok().flatten().map(|x| x.coins).unwrap_or(0),
            None => 0,
        }
    }

    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.cfg_repo = Some(repo);
        self
    }

    async fn settings(&self, guild_id: &str) -> InfluenceSettings {
        match &self.cfg_repo {
            Some(repo) => InfluenceSettings::load(repo.as_ref(), guild_id).await,
            None => InfluenceSettings::default(),
        }
    }

    fn parse_id(id: &str) -> Result<Uuid, DomainError> {
        Uuid::parse_str(id)
            .map_err(|_| DomainError::ValidationError("Identifiant invalide.".into()))
    }
}

#[async_trait]
impl ManageInformationUseCase for ManageInformationService {
    async fn open_investigation(
        &self,
        guild_id: &str,
        initiator_user_id: &str,
        initiator_username: &str,
        target_user_id: &str,
        target_username: &str,
        subject: &str,
    ) -> Result<Investigation, DomainError> {
        let subject = subject.trim();
        validate_non_empty(subject, "Le sujet")?;
        if subject.chars().count() > SUBJECT_MAX {
            return Err(DomainError::ValidationError(format!(
                "Le sujet ne peut pas depasser {SUBJECT_MAX} caracteres"
            )));
        }
        if target_user_id == initiator_user_id {
            return Err(DomainError::ValidationError(
                "Tu ne peux pas enqueter sur toi-meme.".into(),
            ));
        }

        let settings = self.settings(guild_id).await;
        let cost = settings.investigation_cost();
        let initiator = self
            .citizens
            .get_or_create(guild_id, initiator_user_id, initiator_username, 0)
            .await?;
        // Cout en coins (wallet partage).
        let balance = self.money_balance(guild_id, initiator_user_id).await;
        if balance < cost {
            return Err(DomainError::Forbidden(format!(
                "Une enquete coute {cost} coins (tu en as {balance})."
            )));
        }
        match &self.wallet {
            Some(w) => {
                w.debit(guild_id, initiator_user_id, cost, "influence", "Enquete").await?;
            }
            None => {
                self.citizens.adjust_capital(initiator.id, Capital::Money, -cost).await?;
                let _ = self
                    .movements
                    .record(guild_id, initiator.id, Capital::Money, -cost, "Enquete")
                    .await;
            }
        }

        let resolves_at = Utc::now() + Duration::hours(settings.investigation_hours().max(1));
        self.investigations
            .create(NewInvestigation {
                guild_id,
                initiator_id: initiator.id,
                initiator_user_id,
                target_user_id,
                target_username,
                subject,
                resolves_at,
            })
            .await
    }

    async fn list_intel(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<Vec<Information>, DomainError> {
        let start_money = self.settings(guild_id).await.start_money();
        let citizen = self
            .citizens
            .get_or_create(guild_id, user_id, username, start_money)
            .await?;
        self.information.list_secret_for_owner(citizen.id).await
    }

    async fn reveal(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        info_id: &str,
    ) -> Result<RevealOutcome, DomainError> {
        let id = Self::parse_id(info_id)?;
        let start_money = self.settings(guild_id).await.start_money();
        let citizen = self
            .citizens
            .get_or_create(guild_id, user_id, username, start_money)
            .await?;

        let info = self
            .information
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Information introuvable.".into()))?;
        if info.owner_id != citizen.id {
            return Err(DomainError::Forbidden(
                "Cette information ne t'appartient pas.".into(),
            ));
        }
        if info.revealed || info.visibility == Visibility::Public {
            return Err(DomainError::Conflict("Deja revelee.".into()));
        }

        // Bascule ATOMIQUE : si false, une revelation concurrente a deja eu lieu
        // -> on n'applique pas la perte de reputation / le scandale une 2e fois.
        if !self.information.reveal(id).await? {
            return Err(DomainError::Conflict("Deja revelee.".into()));
        }

        // Scandale : la cible perd de la reputation, MODULEE par la CREDIBILITE
        // du revelateur (sa fiabilite). Un accusateur fiable frappe fort ; un
        // menteur notoire (fiabilite negative, deja pris en scandale) fait a
        // peine mal. Facteur = (100 + fiabilite) borne a [50, 150] %.
        let base_loss = self.settings(guild_id).await.scandal_reputation_loss();
        let loss = match &self.rep_dims {
            Some(d) => {
                let reliability = d.get(citizen.id).await.map(|x| x.reliability).unwrap_or(0);
                let pct = (100 + reliability).clamp(50, 150);
                (base_loss * pct / 100).max(0)
            }
            None => base_loss,
        };
        let mut new_target_reputation = None;
        let mut applied_loss = 0;
        if !info.target_user_id.is_empty() {
            let target = self
                .citizens
                .get_or_create(guild_id, &info.target_user_id, &info.target_username, start_money)
                .await?;
            // TRANSPARENCE de la cible : si elle etait deja a decouvert, le
            // scandale fait moins mal (jusqu'a -50 % du malus).
            let effective_loss = match &self.rep_dims {
                Some(dims) => {
                    let transp = dims.get(target.id).await.map(|x| x.transparency).unwrap_or(0);
                    (loss * (100 - transp.clamp(0, 50)) / 100).max(0)
                }
                None => loss,
            };
            applied_loss = effective_loss;
            let new_rep = self
                .citizens
                .adjust_capital(target.id, Capital::Reputation, -effective_loss)
                .await?;
            new_target_reputation = Some(new_rep);
            let _ = self
                .movements
                .record(guild_id, target.id, Capital::Reputation, -effective_loss, "Scandale")
                .await;
            // Reputation multi-dimensionnelle : un scandale entame la FIABILITE
            // et la TRANSPARENCE de la cible (elle cachait quelque chose).
            if let Some(dims) = &self.rep_dims {
                use crate::domain::entities::influence::reputation_dims::ReputationDim;
                let _ = dims.adjust(target.id, ReputationDim::Reliability, -10).await;
                let _ = dims.adjust(target.id, ReputationDim::Transparency, -10).await;
            }
        }

        // L'enqueteur qui revele gagne en NOTORIETE.
        if let Some(dims) = &self.rep_dims {
            use crate::domain::entities::influence::reputation_dims::ReputationDim;
            let _ = dims.adjust(citizen.id, ReputationDim::Notoriety, 5).await;
        }

        let _ = self
            .archives
            .append(
                guild_id,
                "scandal",
                serde_json::json!({
                    "author": username,
                    "target": info.target_username,
                    "content": info.content,
                }),
            )
            .await;

        Ok(RevealOutcome {
            content: info.content,
            target_user_id: info.target_user_id,
            target_username: info.target_username,
            reputation_loss: applied_loss,
            new_target_reputation,
        })
    }

    async fn resolve_due(&self) -> Result<Vec<ResolvedInvestigation>, DomainError> {
        let due = self.investigations.list_due(Utc::now()).await?;
        let mut results = Vec::new();
        for inv in due {
            let success_pct = self.settings(&inv.guild_id).await.investigation_success_pct();
            let roll = rand::thread_rng().gen_range(0..100);
            let success = roll < success_pct;
            let status = if success {
                InvestigationStatus::Reussie
            } else {
                InvestigationStatus::Echouee
            };

            // Reclamation ATOMIQUE : si false, une autre execution (worker vs
            // trigger HTTP) a deja traite cette enquete -> on ne cree pas d'intel
            // en double, on ne rejoue pas le resultat.
            if !self.investigations.resolve(inv.id, status, None).await? {
                continue;
            }

            if success {
                let content = format!(
                    "Révélation compromettante sur {} : {}",
                    if inv.target_username.is_empty() { "la cible" } else { &inv.target_username },
                    inv.subject
                );
                let info_id = self
                    .information
                    .create_secret(NewInformation {
                        guild_id: &inv.guild_id,
                        owner_id: inv.initiator_id,
                        target_user_id: &inv.target_user_id,
                        target_username: &inv.target_username,
                        content: &content,
                    })
                    .await?;
                self.investigations.attach_info(inv.id, info_id).await?;
            }

            results.push(ResolvedInvestigation {
                initiator_user_id: inv.initiator_user_id,
                target_username: inv.target_username,
                subject: inv.subject,
                success,
            });
        }
        Ok(results)
    }
}
