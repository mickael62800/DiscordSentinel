//! Service : consultation des capitaux + conversions.

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::influence::guild_settings::InfluenceSettings;
use crate::domain::entities::influence::capital::Capital;
use crate::domain::entities::influence::conversion::{
    convert, ConversionError, ConversionKind,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::influence::manage_capital::{
    CapitalLine, CapitalOverview, ConversionOutcome, ManageCapitalUseCase,
};
use crate::ports::outbound::influence::citizen_repository::CitizenRepository;
use crate::ports::outbound::influence::movement_repository::MovementRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

/// Nombre de mouvements affiches dans l'historique.
const HISTORY_LIMIT: i64 = 10;

pub struct ManageCapitalService {
    citizens: Arc<dyn CitizenRepository>,
    movements: Arc<dyn MovementRepository>,
    cfg_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl ManageCapitalService {
    pub fn new(
        citizens: Arc<dyn CitizenRepository>,
        movements: Arc<dyn MovementRepository>,
    ) -> Self {
        Self {
            citizens,
            movements,
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
impl ManageCapitalUseCase for ManageCapitalService {
    async fn view(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<CapitalOverview, DomainError> {
        let start_money = self.settings(guild_id).await.start_money();
        let citizen = self
            .citizens
            .get_or_create(guild_id, user_id, username, start_money)
            .await?;
        let c = citizen.capitals;
        let lines = vec![
            CapitalLine { capital: Capital::Influence, value: c.influence },
            CapitalLine { capital: Capital::Money, value: c.money },
            CapitalLine { capital: Capital::Reputation, value: c.reputation },
            CapitalLine { capital: Capital::Information, value: c.information },
            CapitalLine { capital: Capital::Network, value: c.network },
        ];
        let movements = self
            .movements
            .list_recent(citizen.id, HISTORY_LIMIT)
            .await
            .unwrap_or_default();
        Ok(CapitalOverview { lines, movements })
    }

    async fn convert(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        kind: ConversionKind,
        budget: i64,
    ) -> Result<ConversionOutcome, DomainError> {
        if budget <= 0 {
            return Err(DomainError::ValidationError(
                "Le montant doit etre positif.".into(),
            ));
        }
        let settings = self.settings(guild_id).await;
        let rates = settings.conversion_rates();
        let citizen = self
            .citizens
            .get_or_create(guild_id, user_id, username, settings.start_money())
            .await?;

        let source = kind.source();
        let target = kind.target();
        let available = citizen.capitals.get(source);

        let result = convert(kind, budget, available, &rates).map_err(|e| match e {
            ConversionError::InvalidRate => {
                DomainError::Internal("Conversion indisponible (taux non configure).".into())
            }
            ConversionError::BelowMinimum { cost } => DomainError::ValidationError(format!(
                "Il faut au moins {cost} de {} pour obtenir 1 {}.",
                source.label(),
                target.label()
            )),
            ConversionError::Insufficient { available, needed } => DomainError::Forbidden(format!(
                "{} insuffisant : il t'en faut {needed}, tu en as {available}.",
                source.label()
            )),
        })?;

        // Applique le debit puis le credit.
        let reason = format!("Conversion {} -> {}", source.label(), target.label());
        let new_source = self
            .citizens
            .adjust_capital(citizen.id, source, -result.spent)
            .await?;
        let new_target = self
            .citizens
            .adjust_capital(citizen.id, target, result.gained)
            .await?;

        // Trace best-effort (ne fait pas echouer la conversion).
        let _ = self
            .movements
            .record(guild_id, citizen.id, source, -result.spent, &reason)
            .await;
        let _ = self
            .movements
            .record(guild_id, citizen.id, target, result.gained, &reason)
            .await;

        Ok(ConversionOutcome {
            kind,
            spent: result.spent,
            gained: result.gained,
            new_source,
            new_target,
        })
    }
}
