use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{
    calculate_bet_resolution, CoudeBet, NewCoudeBet, RefundSummary,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_bets::{
    ManageCoudeBetsUseCase, PlaceBetOutcome, ResolveBetsOutcome,
};
use crate::ports::inbound::manage_coude_combats::ManageCoudeCombatsUseCase;
use crate::ports::outbound::CoudeBetRepository;

pub struct ManageCoudeBetsService {
    bet_repo: Arc<dyn CoudeBetRepository>,
    combats_uc: Arc<dyn ManageCoudeCombatsUseCase>,
}

impl ManageCoudeBetsService {
    pub fn new(
        bet_repo: Arc<dyn CoudeBetRepository>,
        combats_uc: Arc<dyn ManageCoudeCombatsUseCase>,
    ) -> Self {
        Self { bet_repo, combats_uc }
    }
}

#[async_trait]
impl ManageCoudeBetsUseCase for ManageCoudeBetsService {
    async fn place(&self, new: NewCoudeBet) -> Result<PlaceBetOutcome, DomainError> {
        if new.amount <= 0 {
            return Err(DomainError::ValidationError(
                "Le montant du pari doit etre positif".into(),
            ));
        }

        let combat = self
            .combats_uc
            .get(new.combat_id)
            .await
            .map_err(|_| DomainError::NotFound("Combat introuvable".into()))?;

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

    async fn list_for_combat(&self, combat_id: Uuid) -> Result<Vec<CoudeBet>, DomainError> {
        self.bet_repo.list_for_combat(combat_id).await
    }

    async fn resolve(
        &self,
        combat_id: Uuid,
        winner_id: Option<String>,
    ) -> Result<ResolveBetsOutcome, DomainError> {
        let combat = self.combats_uc.get(combat_id).await?;
        let bets = self.bet_repo.list_for_combat(combat_id).await?;

        let plan = calculate_bet_resolution(
            &bets,
            winner_id.as_deref(),
            &combat.attacker_id,
            &combat.defender_id,
        );

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
        let combat = self.combats_uc.get(combat_id).await?;
        self.bet_repo
            .refund_unresolved(&combat.guild_id, combat_id)
            .await
    }
}
