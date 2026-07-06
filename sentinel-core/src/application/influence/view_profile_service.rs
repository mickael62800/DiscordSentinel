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
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use crate::ports::outbound::influence::citizen_repository::CitizenRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

pub struct ViewProfileService {
    citizens: Arc<dyn CitizenRepository>,
    wallet: Option<Arc<dyn WalletRepository>>,
    cfg_repo: Option<Arc<dyn BotConfigRepository>>,
    rep_dims: Option<
        Arc<dyn crate::ports::outbound::influence::reputation_dims_repository::ReputationDimsRepository>,
    >,
}

impl ViewProfileService {
    pub fn new(citizens: Arc<dyn CitizenRepository>) -> Self {
        Self {
            citizens,
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

    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.cfg_repo = Some(repo);
        self
    }

    /// Branche l'« Argent » sur le wallet partage (`user_wallets.coins`).
    pub fn with_wallet_repo(mut self, repo: Arc<dyn WalletRepository>) -> Self {
        self.wallet = Some(repo);
        self
    }

    /// Solde d'Argent = coins du wallet partage (0 si aucun wallet).
    async fn money_balance(&self, guild_id: &str, user_id: &str, fallback: i64) -> i64 {
        match &self.wallet {
            Some(w) => w.get(guild_id, user_id).await.ok().flatten().map(|x| x.coins).unwrap_or(0),
            None => fallback,
        }
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
            .get_or_create(guild_id, target_user_id, target_username, 0)
            .await?;

        let is_self = viewer_user_id == target_user_id;
        let th = settings.tier_thresholds();
        let capitals = citizen.capitals;
        // Argent = coins du wallet partage (source unique de verite).
        let money = self.money_balance(guild_id, target_user_id, capitals.money).await;

        // Vue d'un capital « generique » (palier + etoiles, chiffre si soi).
        let cap = |value: i64| CapitalView {
            tier: to_tier(value, &th).label(),
            stars: to_tier(value, &th).stars(),
            exact: is_self.then_some(value),
        };

        // Dimensions de reputation : chiffres exacts, seulement sur son propre
        // profil (comme les autres capitaux).
        let reputation_dims = if is_self {
            match &self.rep_dims {
                Some(d) => d.get(citizen.id).await.ok(),
                None => None,
            }
        } else {
            None
        };

        Ok(ProfileView {
            username: citizen.username,
            is_self,
            influence: cap(capitals.influence),
            money: cap(money),
            reputation_tier: to_reputation_tier(capitals.reputation).label(),
            reputation_exact: is_self.then_some(capitals.reputation),
            information: cap(capitals.information),
            network: cap(capitals.network),
            joined_at: citizen.joined_at,
            reputation_dims,
        })
    }
}
