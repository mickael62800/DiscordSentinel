//! Service : cycle de loi (proposition, vote, cloture).

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::application::influence::guild_settings::InfluenceSettings;
use crate::application::validation::validate_non_empty;
use crate::domain::entities::influence::law::{
    clamp_effect_value, law_effect_key, law_effect_label, Law, LawStatus,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::influence::manage_laws::{LawState, ManageLawsUseCase};
use crate::ports::outbound::influence::citizen_repository::CitizenRepository;
use crate::ports::outbound::influence::information_repository::ArchiveRepository;
use crate::ports::outbound::influence::law_repository::LawRepository;
use crate::ports::outbound::influence::motion_repository::VoteRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

const TITLE_MAX: usize = 120;
const BODY_MAX: usize = 1500;

pub struct ManageLawsService {
    citizens: Arc<dyn CitizenRepository>,
    laws: Arc<dyn LawRepository>,
    votes: Arc<dyn VoteRepository>,
    archives: Option<Arc<dyn ArchiveRepository>>,
    cfg_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl ManageLawsService {
    pub fn new(
        citizens: Arc<dyn CitizenRepository>,
        laws: Arc<dyn LawRepository>,
        votes: Arc<dyn VoteRepository>,
    ) -> Self {
        Self {
            citizens,
            laws,
            votes,
            archives: None,
            cfg_repo: None,
        }
    }

    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.cfg_repo = Some(repo);
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

    fn parse_id(law_id: &str) -> Result<Uuid, DomainError> {
        Uuid::parse_str(law_id)
            .map_err(|_| DomainError::ValidationError("Identifiant de loi invalide.".into()))
    }

    async fn state_of(&self, law: Law) -> Result<LawState, DomainError> {
        let tally = self.votes.tally(law.id).await?;
        Ok(LawState { law, tally })
    }
}

#[async_trait]
impl ManageLawsUseCase for ManageLawsService {
    async fn propose(
        &self,
        guild_id: &str,
        author_user_id: &str,
        author_username: &str,
        title: &str,
        body: &str,
        effect_param: Option<&str>,
        effect_value: Option<i64>,
    ) -> Result<LawState, DomainError> {
        let title = title.trim();
        let body = body.trim();
        validate_non_empty(title, "Le titre")?;
        validate_non_empty(body, "Le texte")?;
        if title.chars().count() > TITLE_MAX {
            return Err(DomainError::ValidationError(format!(
                "Le titre ne peut pas depasser {TITLE_MAX} caracteres"
            )));
        }
        if body.chars().count() > BODY_MAX {
            return Err(DomainError::ValidationError(format!(
                "Le texte ne peut pas depasser {BODY_MAX} caracteres"
            )));
        }

        let settings = self.settings(guild_id).await;
        let citizen = self
            .citizens
            .get_or_create(guild_id, author_user_id, author_username, settings.start_money())
            .await?;

        // Effet optionnel : le parametre doit etre dans la whitelist ; la valeur
        // est bornee selon la cle. Sinon la loi reste purement narrative.
        let (effect_key, effect_val): (Option<&str>, Option<i64>) = match effect_param {
            Some(param) => {
                let key = law_effect_key(param).ok_or_else(|| {
                    DomainError::ValidationError(format!("Parametre de loi inconnu : {param}"))
                })?;
                let val = effect_value.ok_or_else(|| {
                    DomainError::ValidationError(
                        "Une valeur est requise pour ce parametre de loi.".into(),
                    )
                })?;
                (Some(key), Some(clamp_effect_value(key, val)))
            }
            None => (None, None),
        };

        let hours = settings.law_debate_hours().max(1);
        let closes_at = Utc::now() + Duration::hours(hours);
        let law = self
            .laws
            .create(guild_id, title, body, citizen.id, closes_at, effect_key, effect_val)
            .await?;
        Ok(LawState {
            law,
            tally: Default::default(),
        })
    }

    async fn vote(
        &self,
        guild_id: &str,
        law_id: &str,
        user_id: &str,
        username: &str,
        choice: crate::domain::entities::influence::vote::VoteChoice,
    ) -> Result<LawState, DomainError> {
        let id = Self::parse_id(law_id)?;
        let law = self
            .laws
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Loi introuvable.".into()))?;
        if law.status != LawStatus::Vote {
            return Err(DomainError::Conflict("Le vote sur cette loi est clos.".into()));
        }

        let settings = self.settings(guild_id).await;
        let citizen = self
            .citizens
            .get_or_create(guild_id, user_id, username, settings.start_money())
            .await?;
        self.votes.upsert(law.id, citizen.id, choice).await?;
        self.state_of(law).await
    }

    async fn get_state(&self, law_id: &str) -> Result<LawState, DomainError> {
        let id = Self::parse_id(law_id)?;
        let law = self
            .laws
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Loi introuvable.".into()))?;
        self.state_of(law).await
    }

    async fn list_active(&self, guild_id: &str) -> Result<Vec<LawState>, DomainError> {
        let laws = self.laws.list_active(guild_id).await?;
        let mut out = Vec::with_capacity(laws.len());
        for law in laws {
            out.push(self.state_of(law).await?);
        }
        Ok(out)
    }

    async fn set_message(
        &self,
        law_id: &str,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), DomainError> {
        let id = Self::parse_id(law_id)?;
        self.laws.set_message(id, channel_id, message_id).await
    }

    async fn close_due(&self) -> Result<Vec<LawState>, DomainError> {
        let due = self.laws.list_due(Utc::now()).await?;
        let mut closed = Vec::new();
        for mut law in due {
            let mut tally = self.votes.tally(law.id).await?;
            // Le financement (lobbying des orgs) s'ajoute au poids des votes.
            tally.pour_weight += law.funding_pour;
            tally.contre_weight += law.funding_contre;
            let status = if tally.is_adopted() {
                LawStatus::Adoptee
            } else {
                LawStatus::Rejetee
            };
            // Cloture ATOMIQUE : si false, une autre execution a deja cloture
            // cette loi -> on n'archive/diffuse pas en double.
            if !self.laws.close(law.id, status).await? {
                continue;
            }
            law.status = status;

            // EFFET REEL : une loi adoptee porteuse d'un effet fixe le reglage
            // gameplay correspondant (whitelist + valeur bornee). C'est le seul
            // endroit qui applique l'effet.
            let mut effect_desc: Option<String> = None;
            if status == LawStatus::Adoptee {
                if let (Some(key), Some(val)) = (law.effect_key.as_deref(), law.effect_value) {
                    let v = clamp_effect_value(key, val);
                    if let Some(repo) = &self.cfg_repo {
                        let _ = repo
                            .set_config(&law.guild_id, "influence-bot", key, &v.to_string())
                            .await;
                    }
                    let label = law_effect_label(key).unwrap_or(key);
                    effect_desc = Some(format!("{label} → {v}"));
                }
            }

            if let Some(arch) = &self.archives {
                let event = if status == LawStatus::Adoptee {
                    "law_adopted"
                } else {
                    "law_rejected"
                };
                let _ = arch
                    .append(
                        &law.guild_id,
                        event,
                        serde_json::json!({
                            "title": law.title,
                            "pour": tally.pour,
                            "contre": tally.contre,
                            "effect": effect_desc,
                        }),
                    )
                    .await;
            }

            closed.push(LawState { law, tally });
        }
        Ok(closed)
    }
}
