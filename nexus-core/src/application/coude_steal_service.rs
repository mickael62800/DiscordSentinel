//! Vol de coins entre membres.
//!
//! Toutes les valeurs qui font l'equilibre — chance de reussite, part volee,
//! penalite d'echec, solde minimum d'une cible — viennent desormais de la
//! configuration du serveur. Elles etaient en dur : regler un vol trop
//! punitif demandait de recompiler, autrement dit de ne jamais le regler.
//!
//! Les defauts de `CoudeConfig` reproduisent exactement les anciennes
//! constantes, donc rien ne change tant que personne ne touche a la
//! configuration.

use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;

use crate::application::economy_config::load_coude;
use crate::domain::errors::DomainError;
use crate::ports::{
    inbound::coude_steal::{CoudeStealUseCase, StealResult},
    outbound::{
        coude_steal_repository::CoudeStealRepository,
        system::bot_config_repository::BotConfigRepository,
    },
};

pub struct CoudeStealService {
    repo: Arc<dyn CoudeStealRepository>,
    config_repo: Arc<dyn BotConfigRepository>,
}

impl CoudeStealService {
    pub fn new(
        repo: Arc<dyn CoudeStealRepository>,
        config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self { repo, config_repo }
    }
}

#[async_trait]
impl CoudeStealUseCase for CoudeStealService {
    async fn steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        is_fourbe: bool,
    ) -> Result<StealResult, DomainError> {
        if thief_id == victim_id {
            return Err(DomainError::Validation(
                "impossible de se voler soi-meme".into(),
            ));
        }

        let cfg = load_coude(&self.config_repo, guild_id).await?;
        if !cfg.steal_enabled {
            return Err(DomainError::Validation(
                "les vols sont desactives sur ce serveur".into(),
            ));
        }

        let (thief, victim) = self.repo.balances(guild_id, thief_id, victim_id).await?;

        // Plancher de pauvrete : sans lui, on peut achever quelqu'un qui n'a
        // deja plus rien. Ca ne rapporte presque rien et ca degoute.
        if victim < cfg.steal_min_victim_coins {
            return Err(DomainError::Validation(format!(
                "cible trop pauvre (moins de {} coins)",
                cfg.steal_min_victim_coins
            )));
        }

        let success = rand::thread_rng().gen_range(0..100) < cfg.steal_chance(is_fourbe);
        let amount = if success {
            cfg.steal_gain(victim)
        } else {
            cfg.steal_penalty(thief)
        };

        self.repo
            .transfer(guild_id, thief_id, victim_id, amount, success)
            .await?;

        Ok(StealResult { success, amount })
    }
}
