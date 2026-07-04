//! Service : votes binaires sur motions.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::application::influence::guild_settings::InfluenceSettings;
use crate::application::validation::validate_non_empty;
use crate::domain::entities::influence::motion::{Motion, MotionStatus};
use crate::domain::entities::influence::vote::{Tally, VoteChoice};
use crate::domain::errors::DomainError;
use crate::ports::inbound::influence::manage_votes::{ManageVotesUseCase, MotionState};
use crate::ports::outbound::influence::citizen_repository::CitizenRepository;
use crate::ports::outbound::influence::membership_repository::MembershipRepository;
use crate::ports::outbound::influence::motion_repository::{MotionRepository, VoteRepository};
use crate::ports::outbound::influence::organization_repository::OrganizationRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

const TITLE_MAX: usize = 200;

pub struct ManageVotesService {
    citizens: Arc<dyn CitizenRepository>,
    orgs: Arc<dyn OrganizationRepository>,
    memberships: Arc<dyn MembershipRepository>,
    motions: Arc<dyn MotionRepository>,
    votes: Arc<dyn VoteRepository>,
    cfg_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl ManageVotesService {
    pub fn new(
        citizens: Arc<dyn CitizenRepository>,
        orgs: Arc<dyn OrganizationRepository>,
        memberships: Arc<dyn MembershipRepository>,
        motions: Arc<dyn MotionRepository>,
        votes: Arc<dyn VoteRepository>,
    ) -> Self {
        Self {
            citizens,
            orgs,
            memberships,
            motions,
            votes,
            cfg_repo: None,
        }
    }

    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.cfg_repo = Some(repo);
        self
    }

    async fn start_money(&self, guild_id: &str) -> i64 {
        match &self.cfg_repo {
            Some(repo) => InfluenceSettings::load(repo.as_ref(), guild_id).await.start_money(),
            None => InfluenceSettings::default().start_money(),
        }
    }

    fn parse_id(motion_id: &str) -> Result<Uuid, DomainError> {
        Uuid::parse_str(motion_id)
            .map_err(|_| DomainError::ValidationError("Identifiant de motion invalide.".into()))
    }

    async fn org_name(&self, org_id: Uuid) -> String {
        self.orgs
            .get(org_id)
            .await
            .ok()
            .flatten()
            .map(|o| o.name)
            .unwrap_or_else(|| "?".to_string())
    }

    async fn state(&self, motion: Motion) -> Result<MotionState, DomainError> {
        let tally = self.votes.tally(motion.id).await?;
        let org_name = self.org_name(motion.org_id).await;
        Ok(MotionState {
            motion,
            org_name,
            tally,
        })
    }
}

#[async_trait]
impl ManageVotesUseCase for ManageVotesService {
    async fn create_motion(
        &self,
        guild_id: &str,
        org_name: &str,
        creator_user_id: &str,
        creator_username: &str,
        title: &str,
    ) -> Result<MotionState, DomainError> {
        let title = title.trim();
        validate_non_empty(title, "Le sujet")?;
        if title.chars().count() > TITLE_MAX {
            return Err(DomainError::ValidationError(format!(
                "Le sujet ne peut pas depasser {TITLE_MAX} caracteres"
            )));
        }

        let start_money = self.start_money(guild_id).await;
        let citizen = self
            .citizens
            .get_or_create(guild_id, creator_user_id, creator_username, start_money)
            .await?;
        let org = self
            .orgs
            .find_by_name(guild_id, org_name)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Aucune organisation « {org_name} »")))?;

        if self.memberships.get(org.id, citizen.id).await?.is_none() {
            return Err(DomainError::Forbidden(
                "Tu dois etre membre de l'organisation pour y ouvrir un vote.".into(),
            ));
        }

        let motion = self
            .motions
            .create(guild_id, org.id, title, citizen.id)
            .await?;
        Ok(MotionState {
            motion,
            org_name: org.name,
            tally: Tally::default(),
        })
    }

    async fn cast_vote(
        &self,
        guild_id: &str,
        motion_id: &str,
        user_id: &str,
        username: &str,
        choice: VoteChoice,
    ) -> Result<MotionState, DomainError> {
        let id = Self::parse_id(motion_id)?;
        let motion = self
            .motions
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Motion introuvable.".into()))?;
        if motion.status != MotionStatus::Ouverte {
            return Err(DomainError::Conflict("Ce vote est clos.".into()));
        }

        let start_money = self.start_money(guild_id).await;
        let citizen = self
            .citizens
            .get_or_create(guild_id, user_id, username, start_money)
            .await?;
        if self.memberships.get(motion.org_id, citizen.id).await?.is_none() {
            return Err(DomainError::Forbidden(
                "Seuls les membres de l'organisation peuvent voter.".into(),
            ));
        }

        self.votes.upsert(motion.id, citizen.id, choice).await?;
        self.state(motion).await
    }

    async fn close_motion(
        &self,
        guild_id: &str,
        motion_id: &str,
        user_id: &str,
    ) -> Result<MotionState, DomainError> {
        let id = Self::parse_id(motion_id)?;
        let mut motion = self
            .motions
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Motion introuvable.".into()))?;
        if motion.status != MotionStatus::Ouverte {
            return Err(DomainError::Conflict("Ce vote est deja clos.".into()));
        }

        // Seul l'auteur peut cloturer.
        let citizen = self
            .citizens
            .get(guild_id, user_id)
            .await?
            .ok_or_else(|| DomainError::Forbidden("Seul l'auteur peut cloturer ce vote.".into()))?;
        if citizen.id != motion.created_by {
            return Err(DomainError::Forbidden(
                "Seul l'auteur peut cloturer ce vote.".into(),
            ));
        }

        let tally = self.votes.tally(motion.id).await?;
        let status = if tally.is_adopted() {
            MotionStatus::Adoptee
        } else {
            MotionStatus::Rejetee
        };
        self.motions.set_status(motion.id, status).await?;
        motion.status = status;

        let org_name = self.org_name(motion.org_id).await;
        Ok(MotionState {
            motion,
            org_name,
            tally,
        })
    }

    async fn get_state(
        &self,
        _guild_id: &str,
        motion_id: &str,
    ) -> Result<MotionState, DomainError> {
        let id = Self::parse_id(motion_id)?;
        let motion = self
            .motions
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Motion introuvable.".into()))?;
        self.state(motion).await
    }
}
