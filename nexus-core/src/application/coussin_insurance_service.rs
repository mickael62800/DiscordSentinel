//! Assurance : protege d'une perte, sauf quand elle se revele etre une
//! arnaque — ce qui fait tout le sel de la mecanique.

use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;

use crate::application::economy_config::load_coussin;
use crate::domain::errors::DomainError;
use crate::ports::{
    inbound::coussin_insurance::CoussinInsuranceUseCase,
    outbound::{
        coussin_insurance_repository::{CoussinInsurance, CoussinInsuranceRepository},
        system::bot_config_repository::BotConfigRepository,
    },
};

/// Une assurance sur vingt est une arnaque. Volontairement NON configurable :
/// c'est la blague de la mecanique, pas un curseur d'equilibrage. La rendre
/// reglable inviterait a la mettre a 0 — et l'assurance deviendrait un achat
/// sans histoire — ou a 100, ce qui n'est plus une assurance.
const SCAM_PERCENT: u32 = 5;

pub struct CoussinInsuranceService {
    repo: Arc<dyn CoussinInsuranceRepository>,
    config_repo: Arc<dyn BotConfigRepository>,
}

impl CoussinInsuranceService {
    pub fn new(
        repo: Arc<dyn CoussinInsuranceRepository>,
        config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self { repo, config_repo }
    }
}

#[async_trait]
impl CoussinInsuranceUseCase for CoussinInsuranceService {
    async fn buy(&self, guild_id: &str, user_id: &str) -> Result<CoussinInsurance, DomainError> {
        let cfg = load_coussin(&self.config_repo, guild_id).await?;
        cfg.ensure_enabled()?;
        if !cfg.insurance_enabled {
            return Err(DomainError::Validation(
                "l'assurance n'est pas disponible sur ce serveur".into(),
            ));
        }

        let is_scam = rand::thread_rng().gen_range(0..100) < SCAM_PERCENT;
        self.repo
            .buy(
                guild_id,
                user_id,
                is_scam,
                cfg.insurance_cost.max(1),
                cfg.insurance_duration_minutes.max(1),
            )
            .await
    }

    /// Lecture seule : consultable meme si l'assurance a ete desactivee
    /// depuis. Quelqu'un qui en a achete une doit pouvoir la voir jusqu'a son
    /// terme.
    async fn active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoussinInsurance>, DomainError> {
        self.repo.active(guild_id, user_id).await
    }
}
