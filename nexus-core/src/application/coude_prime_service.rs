//! Primes posees sur la tete d'un joueur.

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::economy_config::load_coude;
use crate::domain::errors::DomainError;
use crate::ports::{
    inbound::coude_prime::CoudePrimeUseCase,
    outbound::{
        coude_prime_repository::CoudePrimeRepository,
        system::bot_config_repository::BotConfigRepository,
    },
};

pub struct CoudePrimeService {
    repo: Arc<dyn CoudePrimeRepository>,
    config_repo: Arc<dyn BotConfigRepository>,
}

impl CoudePrimeService {
    pub fn new(
        repo: Arc<dyn CoudePrimeRepository>,
        config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self { repo, config_repo }
    }
}

#[async_trait]
impl CoudePrimeUseCase for CoudePrimeService {
    async fn place(
        &self,
        guild_id: &str,
        target_id: &str,
        target_name: &str,
        placer_id: &str,
        placer_name: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        // Regle universelle, independante de toute configuration : se mettre
        // une prime sur la tete n'a aucun sens.
        if target_id == placer_id {
            return Err(DomainError::Validation(
                "impossible de poser une prime sur soi".into(),
            ));
        }

        let cfg = load_coude(&self.config_repo, guild_id).await?;
        if !cfg.prime_enabled {
            return Err(DomainError::Validation(
                "les primes sont desactivees sur ce serveur".into(),
            ));
        }
        if amount < cfg.prime_min {
            return Err(DomainError::Validation(format!(
                "la prime minimum est de {} coins",
                cfg.prime_min
            )));
        }
        // 0 = pas de plafond.
        if cfg.prime_max > 0 && amount > cfg.prime_max {
            return Err(DomainError::Validation(format!(
                "la prime maximum est de {} coins",
                cfg.prime_max
            )));
        }

        self.repo
            .place(guild_id, target_id, target_name, placer_id, placer_name, amount)
            .await
    }
}
