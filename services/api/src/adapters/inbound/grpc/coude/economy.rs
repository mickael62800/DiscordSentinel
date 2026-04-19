//! Handler gRPC du service `CoudeEconomyService`.
//!
//! Couvre casino (win/loss/faillite + counters), transferts de coins,
//! record de vol atomique. Delegue a `ManageCoudeEconomyUseCase`.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use sentinel_proto::coude::v1 as proto;
use sentinel_proto::coude::v1::coude_economy_service_server::CoudeEconomyService;

use crate::adapters::inbound::grpc::coude::taunt_event_to_proto;
use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::ports::inbound::ManageCoudeEconomyUseCase;

pub struct CoudeEconomyGrpc {
    pub uc: Arc<dyn ManageCoudeEconomyUseCase>,
}

#[tonic::async_trait]
impl CoudeEconomyService for CoudeEconomyGrpc {
    async fn transfer(
        &self,
        request: Request<proto::TransferRequest>,
    ) -> Result<Response<proto::TransferResponse>, Status> {
        let req = request.into_inner();
        let taunts = self
            .uc
            .transfer(&req.guild_id, &req.from_id, &req.to_id, req.amount)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::TransferResponse {
            taunt_events: taunts.into_iter().map(taunt_event_to_proto).collect(),
        }))
    }

    async fn steal(
        &self,
        request: Request<proto::StealRequest>,
    ) -> Result<Response<proto::StealResponse>, Status> {
        let req = request.into_inner();
        let outcome = self
            .uc
            .steal(&req.guild_id, &req.thief_id, &req.victim_id, req.amount)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::StealResponse {
            stolen: outcome.stolen,
            taunt_events: outcome
                .taunt_events
                .into_iter()
                .map(taunt_event_to_proto)
                .collect(),
        }))
    }

    async fn steal_fail_penalty(
        &self,
        request: Request<proto::StealFailPenaltyRequest>,
    ) -> Result<Response<proto::StealFailPenaltyResponse>, Status> {
        let req = request.into_inner();
        let (lost, taunts) = self
            .uc
            .steal_fail_penalty(&req.guild_id, &req.thief_id, req.amount)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::StealFailPenaltyResponse {
            lost,
            taunt_events: taunts.into_iter().map(taunt_event_to_proto).collect(),
        }))
    }

    async fn record_casino_win(
        &self,
        request: Request<proto::RecordCasinoWinRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .record_casino_win(&req.guild_id, &req.user_id, req.gain)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn record_casino_loss(
        &self,
        request: Request<proto::RecordCasinoLossRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .record_casino_loss(&req.guild_id, &req.user_id, req.lost)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn record_casino_faillite(
        &self,
        request: Request<proto::RecordCasinoFailliteRequest>,
    ) -> Result<Response<proto::RecordCasinoFailliteResponse>, Status> {
        let req = request.into_inner();
        let cleared = self
            .uc
            .record_casino_faillite(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::RecordCasinoFailliteResponse {
            cleared_coins: cleared,
        }))
    }

    async fn count_casino_today(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::Int64Value>, Status> {
        let req = request.into_inner();
        let v = self
            .uc
            .count_casino_today(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Int64Value { value: v }))
    }

    async fn sum_casino_gains_today(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::Int64Value>, Status> {
        let req = request.into_inner();
        let v = self
            .uc
            .sum_casino_gains_today(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Int64Value { value: v }))
    }

    async fn count_steal_today(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::Int64Value>, Status> {
        let req = request.into_inner();
        let v = self
            .uc
            .count_steal_today(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Int64Value { value: v }))
    }
}

