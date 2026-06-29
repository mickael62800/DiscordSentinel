use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::application::coude::guild_settings::GuildSettings;
use crate::domain::entities::coude::bet::calculate_bet_resolution;
use crate::domain::entities::coude::bet::Bet;
use crate::domain::entities::coude::bet::NewCoudeBet;
use crate::domain::entities::coude::bet::RefundSummary;
use crate::domain::entities::coude::safety_net::boost_bet_gain_with_multiplier as safety_net_boost_bet_gain_with_multiplier;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_bets::ManageCoudeBetsUseCase;
use crate::ports::inbound::coude::manage_bets::PlaceBetOutcome;
use crate::ports::inbound::coude::manage_bets::ResolveBetsOutcome;
use crate::ports::outbound::coude::bet_repository::BetRepository;
use crate::ports::outbound::coude::combat_query_repository::CombatQueryRepository;
use crate::ports::outbound::coude::safety_net_repository::SafetyNetRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
pub struct ManageCoudeBetsService {
    bet_repo: Arc<dyn BetRepository>,
    combat_query: Arc<dyn CombatQueryRepository>,
    safety_net_repo: Option<Arc<dyn SafetyNetRepository>>,
    bot_config_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl ManageCoudeBetsService {
    pub fn new(
        bet_repo: Arc<dyn BetRepository>,
        combat_query: Arc<dyn CombatQueryRepository>,
    ) -> Self {
        Self {
            bet_repo,
            combat_query,
            safety_net_repo: None,
            bot_config_repo: None,
        }
    }

    /// Branche le repo du filet de securite (cf. COUPE_AMELIORATIONS 4.4)
    /// pour booster les paris gagnants des joueurs en phase de
    /// recuperation (multiplicateur configurable, default x1.5).
    pub fn with_safety_net_repo(mut self, repo: Arc<dyn SafetyNetRepository>) -> Self {
        self.safety_net_repo = Some(repo);
        self
    }

    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.bot_config_repo = Some(repo);
        self
    }

    async fn bettor_has_safety_net(&self, guild_id: &str, user_id: &str) -> bool {
        let Some(repo) = &self.safety_net_repo else {
            return false;
        };
        matches!(repo.get_active(guild_id, user_id).await, Ok(Some(_)))
    }
}

#[async_trait]
impl ManageCoudeBetsUseCase for ManageCoudeBetsService {
    async fn place(&self, new: NewCoudeBet) -> Result<PlaceBetOutcome, DomainError> {
        crate::application::validation::validate_positive(new.amount, "Le montant du pari")?;

        let combat = self.combat_query.get(new.combat_id).await?;

        if combat.status != "betting" {
            return Err(DomainError::ValidationError(
                "Les paris ne sont pas ouverts pour ce combat".into(),
            ));
        }

        if new.bettor_id == combat.attacker_id || new.bettor_id == combat.defender_id {
            return Err(DomainError::ValidationError(
                "Un participant ne peut pas parier sur son propre combat".into(),
            ));
        }

        let taunt_events = self.bet_repo.place(new).await?;
        Ok(PlaceBetOutcome { taunt_events })
    }

    async fn list_for_combat(&self, combat_id: Uuid) -> Result<Vec<Bet>, DomainError> {
        self.bet_repo.list_for_combat(combat_id).await
    }

    async fn resolve(
        &self,
        combat_id: Uuid,
        winner_id: Option<String>,
    ) -> Result<ResolveBetsOutcome, DomainError> {
        let combat = self.combat_query.get(combat_id).await?;
        let bets = self.bet_repo.list_for_combat(combat_id).await?;

        let mut plan = calculate_bet_resolution(
            &bets,
            winner_id.as_deref(),
            &combat.attacker_id,
            &combat.defender_id,
        );

        // Filet de securite (cf. COUPE_AMELIORATIONS 4.4) : pour chaque
        // parieur gagnant qui a un filet actif, on boost son payout x1.5.
        // Pre-fetch les filets actifs en bulk via list_active pour eviter
        // N requetes (ok pour 5-50 paris par combat).
        if self.safety_net_repo.is_some() {
            let mult = match &self.bot_config_repo {
                Some(repo) => GuildSettings::load(&**repo, &combat.guild_id)
                    .await
                    .get_percent_ratio("safety_net_bet_gain_percent", 150),
                None => 1.5,
            };
            for p in plan.payouts.iter_mut() {
                if !p.won || p.payout <= 0 {
                    continue;
                }
                if self
                    .bettor_has_safety_net(&combat.guild_id, &p.bettor_id)
                    .await
                {
                    p.payout = safety_net_boost_bet_gain_with_multiplier(p.payout, true, mult);
                }
            }
        }

        if plan.payouts.is_empty() {
            // Rien à faire côté DB : pas de paris sur ce combat.
            return Ok(ResolveBetsOutcome {
                plan,
                taunt_events: Vec::new(),
            });
        }

        let taunt_events = self
            .bet_repo
            .apply_resolution(&combat.guild_id, plan.clone())
            .await?;
        Ok(ResolveBetsOutcome { plan, taunt_events })
    }

    async fn refund(&self, combat_id: Uuid) -> Result<RefundSummary, DomainError> {
        let combat = self.combat_query.get(combat_id).await?;
        self.bet_repo
            .refund_unresolved(&combat.guild_id, combat_id)
            .await
    }
}

#[cfg(test)]
#[path = "tests/manage.rs"]
mod tests;
