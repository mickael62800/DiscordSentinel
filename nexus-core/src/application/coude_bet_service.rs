//! Paris sur les combats.
//!
//! Les bornes viennent de la configuration du serveur : un pari minimum trop
//! bas noie le salon de mises symboliques, un gain trop genereux vide le jeu
//! de ses combats — il devient plus rentable de parier que de se battre.

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::economy_config::load_coude;
use crate::domain::errors::DomainError;
use crate::ports::{
    inbound::coude_bet::CoudeBetUseCase,
    outbound::{
        coude_bet_repository::CoudeBetRepository,
        system::bot_config_repository::BotConfigRepository,
    },
};

pub struct CoudeBetService {
    repo: Arc<dyn CoudeBetRepository>,
    config_repo: Arc<dyn BotConfigRepository>,
}

impl CoudeBetService {
    pub fn new(
        repo: Arc<dyn CoudeBetRepository>,
        config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self { repo, config_repo }
    }
}

#[async_trait]
impl CoudeBetUseCase for CoudeBetService {
    async fn place(
        &self,
        guild: &str,
        combat: uuid::Uuid,
        bettor: &str,
        name: &str,
        backed: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        let cfg = load_coude(&self.config_repo, guild).await?;
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
