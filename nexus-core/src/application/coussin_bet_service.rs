//! Paris sur les combats.
//!
//! Les bornes viennent de la configuration du serveur : un pari minimum trop
//! bas noie le salon de mises symboliques, un gain trop genereux vide le jeu
//! de ses combats — il devient plus rentable de parier que de se battre.

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::economy_config::load_coussin;
use crate::domain::errors::DomainError;
use crate::ports::{
    inbound::coussin_bet::CoussinBetUseCase,
    outbound::{
        coussin_bet_repository::CoussinBetRepository,
        system::bot_config_repository::BotConfigRepository,
    },
};

pub struct CoussinBetService {
    repo: Arc<dyn CoussinBetRepository>,
    config_repo: Arc<dyn BotConfigRepository>,
}

impl CoussinBetService {
    pub fn new(
        repo: Arc<dyn CoussinBetRepository>,
        config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self { repo, config_repo }
    }
}

#[async_trait]
impl CoussinBetUseCase for CoussinBetService {
    async fn place(
        &self,
        guild: &str,
        combat: uuid::Uuid,
        bettor: &str,
        name: &str,
        backed: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        let cfg = load_coussin(&self.config_repo, guild).await?;
        if !cfg.bet_enabled {
            return Err(DomainError::Validation(
                "les paris sont desactives sur ce serveur".into(),
            ));
        }
        if amount < cfg.bet_min {
            return Err(DomainError::Validation(format!(
                "le pari minimum est de {} coins",
                cfg.bet_min
            )));
        }

        self.repo
            .place(guild, combat, bettor, name, backed, amount)
            .await
    }
}
