//! Handler gRPC du service `CoudeBetsService`.
//!
//! Gere les paris pari-mutuel sur les combats : place, list, resolve
//! (avec split 15 % commission combattants / 85 % parieurs gagnants),
//! refund. Delegue a `ManageCoudeBetsUseCase`.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use sentinel_proto::coude::v1 as proto;
use sentinel_proto::coude::v1::coude_bets_service_server::CoudeBetsService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::{
    BetPayout, BetResolutionPlan, CoudeBet, CoudeFighterBetBonus, NewCoudeBet, RefundSummary,
};
use crate::ports::inbound::ManageCoudeBetsUseCase;

use super::{parse_uuid, taunt_event_to_proto};

pub struct CoudeBetsGrpc {
    pub uc: Arc<dyn ManageCoudeBetsUseCase>,
}

pub(super) fn bet_to_proto(b: CoudeBet) -> proto::CoudeBet {
    proto::CoudeBet {
        id: b.id.to_string(),
        guild_id: b.guild_id,
        combat_id: b.combat_id.to_string(),
        bettor_id: b.bettor_id,
        bettor_name: b.bettor_name,
        backed_id: b.backed_id,
        amount: b.amount,
        won: b.won,
        payout: b.payout,
    }
}

pub(super) fn bet_payout_to_proto(p: BetPayout) -> proto::BetPayout {
    proto::BetPayout {
        bet_id: p.bet_id.to_string(),
        bettor_id: p.bettor_id,
        bettor_name: p.bettor_name,
        backed_id: p.backed_id,
        amount_bet: p.amount_bet,
        payout: p.payout,
        won: p.won,
    }
}

pub(super) fn fighter_bonus_to_proto(b: CoudeFighterBetBonus) -> proto::FighterBetBonus {
    proto::FighterBetBonus {
        winner_id: b.winner_id,
        winner_bonus: b.winner_bonus,
        loser_id: b.loser_id,
        loser_bonus: b.loser_bonus,
        total_pot: b.total_pot,
    }
}

pub(super) fn bet_resolution_plan_to_proto(p: BetResolutionPlan) -> proto::BetResolutionPlan {
    proto::BetResolutionPlan {
        payouts: p.payouts.into_iter().map(bet_payout_to_proto).collect(),
        fighter_bonus: p.fighter_bonus.map(fighter_bonus_to_proto),
    }
}

pub(super) fn refund_summary_to_proto(s: RefundSummary) -> proto::RefundSummary {
    proto::RefundSummary {
        refunded_count: s.refunded_count as u64,
        refunded_total: s.refunded_total,
    }
}

#[tonic::async_trait]
impl CoudeBetsService for CoudeBetsGrpc {
    async fn place(
        &self,
        request: Request<proto::PlaceBetRequest>,
    ) -> Result<Response<proto::PlaceBetResponse>, Status> {
        let req = request.into_inner();
        let combat_id = parse_uuid(&req.combat_id)?;
        let outcome = self
            .uc
            .place(NewCoudeBet {
                guild_id: req.guild_id,
                combat_id,
                bettor_id: req.bettor_id,
                bettor_name: req.bettor_name,
                backed_id: req.backed_id,
                amount: req.amount,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::PlaceBetResponse {
            taunt_events: outcome
                .taunt_events
                .into_iter()
                .map(taunt_event_to_proto)
                .collect(),
        }))
    }

    async fn list_for_combat(
        &self,
        request: Request<proto::ListForCombatRequest>,
    ) -> Result<Response<proto::BetList>, Status> {
        let combat_id = parse_uuid(&request.into_inner().combat_id)?;
        let list = self
            .uc
            .list_for_combat(combat_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::BetList {
            bets: list.into_iter().map(bet_to_proto).collect(),
        }))
    }

    async fn resolve(
        &self,
        request: Request<proto::ResolveBetsRequest>,
    ) -> Result<Response<proto::ResolveBetsResponse>, Status> {
        let req = request.into_inner();
        let combat_id = parse_uuid(&req.combat_id)?;
        let outcome = self
            .uc
            .resolve(combat_id, req.winner_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ResolveBetsResponse {
            plan: Some(bet_resolution_plan_to_proto(outcome.plan)),
            taunt_events: outcome
                .taunt_events
                .into_iter()
                .map(taunt_event_to_proto)
                .collect(),
        }))
    }

    async fn refund(
        &self,
        request: Request<proto::RefundBetsRequest>,
    ) -> Result<Response<proto::RefundSummary>, Status> {
        let combat_id = parse_uuid(&request.into_inner().combat_id)?;
        let s = self
            .uc
            .refund(combat_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(refund_summary_to_proto(s)))
    }
}

