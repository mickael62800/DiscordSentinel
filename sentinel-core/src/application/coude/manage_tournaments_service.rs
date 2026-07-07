//! Use case tournoi hebdomadaire "Coup de Coude" : assemble le classement
//! courant (rangs + pseudos) et estime le prize pool a partir de la caisse.
//! Toute la regle metier vit ici ; le SQL vit dans `TournamentRepository`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::application::coude::guild_settings::load_economy_config;
use crate::domain::entities::coude::tournament::build_standings;
use crate::domain::entities::coude::tournament::current_week_bounds;
use crate::domain::entities::coude::tournament::estimate_tournament_prize_pool;
use crate::domain::entities::coude::tournament::CurrentTournament;
use crate::domain::entities::coude::tournament::PastTournament;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_tournaments::ManageTournamentsUseCase;
use crate::ports::outbound::coude::tournament_repository::TournamentRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

/// Nombre de membres affiches dans le classement du tournoi courant.
const STANDINGS_LIMIT: i64 = 10;
/// Nombre de tournois passes renvoyes dans l'historique.
const HISTORY_LIMIT: i64 = 20;

pub struct ManageTournamentsService {
    repo: Arc<dyn TournamentRepository>,
    config: Arc<dyn BotConfigRepository>,
}

impl ManageTournamentsService {
    pub fn new(
        repo: Arc<dyn TournamentRepository>,
        config: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self { repo, config }
    }
}

#[async_trait]
impl ManageTournamentsUseCase for ManageTournamentsService {
    async fn current_tournament(
        &self,
        guild_id: &str,
    ) -> Result<CurrentTournament, DomainError> {
        let (week_start, week_end) = current_week_bounds();

        let net_gains = self
            .repo
            .weekly_net_gains(guild_id, week_start, week_end, STANDINGS_LIMIT)
            .await?;

        // Resolution des pseudos en une seule requete, puis assemblage pur.
        let user_ids: Vec<String> = net_gains.iter().map(|(id, _)| id.clone()).collect();
        let usernames: HashMap<String, String> = self
            .repo
            .usernames(guild_id, &user_ids)
            .await?
            .into_iter()
            .collect();
        let standings = build_standings(net_gains, |id| usernames.get(id).cloned());

        // Prize pool estime : `tournament_prize_pool_pct` % de la caisse
        // communautaire (defaut 10%), reglable par serveur.
        let cashbox = self.repo.cashbox_balance(guild_id).await?;
        let econ = load_economy_config(self.config.as_ref(), guild_id).await;
        let prize_pool_estimated = estimate_tournament_prize_pool(cashbox, &econ);

        Ok(CurrentTournament {
            guild_id: guild_id.to_string(),
            week_start,
            week_end,
            prize_pool_estimated,
            standings,
        })
    }

    async fn tournament_history(
        &self,
        guild_id: &str,
    ) -> Result<Vec<PastTournament>, DomainError> {
        self.repo.list_past_tournaments(guild_id, HISTORY_LIMIT).await
    }
}
