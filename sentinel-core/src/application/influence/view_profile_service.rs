//! Service : consultation de profil avec application des paliers narratifs.
//!
//! C'est ici que vit la decision « chiffre exact (soi) vs palier (tiers) »,
//! cœur du principe de l'ARCHITECTURE.md.

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::influence::guild_settings::InfluenceSettings;
use crate::domain::entities::influence::tier::{to_reputation_tier, to_tier};
use crate::domain::errors::DomainError;
use crate::ports::inbound::influence::view_profile::{
    CapitalView, ProfileView, ViewProfileUseCase,
};
use crate::ports::outbound::influence::citizen_repository::CitizenRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

pub struct ViewProfileService {
    citizens: Arc<dyn CitizenRepository>,
    cfg_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl ViewProfileService {
    pub fn new(citizens: Arc<dyn CitizenRepository>) -> Self {
        Self {
            citizens,
            cfg_repo: None,
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
}

#[async_trait]
impl ViewProfileUseCase for ViewProfileService {
    async fn view(
        &self,
        guild_id: &str,
        viewer_user_id: &str,
        target_user_id: &str,
        target_username: &str,
    ) -> Result<ProfileView, DomainError> {
        let settings = self.settings(guild_id).await;
        let citizen = self
            .citizens
            .get_or_create(guild_id, target_user_id, target_username, settings.start_money())
            .await?;

        let is_self = viewer_user_id == target_user_id;
        let th = settings.tier_thresholds();
        let capitals = citizen.capitals;

        // Vue d'un capital « generique » (palier + etoiles, chiffre si soi).
        let cap = |value: i64| CapitalView {
            tier: to_tier(value, &th).label(),
            stars: to_tier(value, &th).stars(),
            exact: is_self.then_some(value),
        };

        Ok(ProfileView {
            username: citizen.username,
            is_self,
            influence: cap(capitals.influence),
            money: cap(capitals.money),
            reputation_tier: to_reputation_tier(capitals.reputation).label(),
            reputation_exact: is_self.then_some(capitals.reputation),
            information: cap(capitals.information),
            network: cap(capitals.network),
            joined_at: citizen.joined_at,
        })
    }
}
