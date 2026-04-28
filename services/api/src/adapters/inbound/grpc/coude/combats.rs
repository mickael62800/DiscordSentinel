//! Handler gRPC du service `CoudeCombatsService`.
//!
//! Gere les combats Coup de Coude : CRUD, resolution (batch + instant),
//! expiration, set_betting, defender_special. Delegue aux use cases
//! correspondants. Inclut aussi le helper `taunt_event_to_proto`
//! utilise par la reponse `ResolveCombatNow` qui peut contenir des
//! TauntEvents (Phase 9 Part D).

use std::sync::Arc;

use tonic::Request;
use tonic::Response;
use tonic::Status;
use sentinel_proto::coude::v1 as proto;
use sentinel_proto::coude::v1::coude_combats_service_server::CoudeCombatsService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::coude::combat::CombatResolution;
use crate::domain::entities::coude::combat::CoudeCombat;
use crate::domain::entities::coude::combat::NewCoudeCombat;
use crate::ports::inbound::coude::manage_combats::ManageCoudeCombatsUseCase;

use super::parse_uuid;
use super::taunt_event_to_proto;
pub struct CoudeCombatsGrpc {
    pub uc: Arc<dyn ManageCoudeCombatsUseCase>,
    pub resolve_batch_uc: Arc<dyn crate::ports::inbound::coude::resolve_betting_batch::ResolveBettingBatchUseCase>,
    pub expire_batch_uc: Arc<dyn crate::ports::inbound::coude::expire_combats_batch::ExpireCombatsBatchUseCase>,
    pub resolve_now_uc: Arc<dyn crate::ports::inbound::coude::resolve_combat_now::ResolveCombatNowUseCase>,
}

#[cfg(test)]
#[path = "tests/combats.rs"]
mod tests;

pub(super) fn combat_to_proto(c: CoudeCombat) -> proto::CoudeCombat {
    proto::CoudeCombat {
        id: c.id.to_string(),
        guild_id: c.guild_id,
        channel_id: c.channel_id,
        attacker_id: c.attacker_id,
        attacker_name: c.attacker_name,
        defender_id: c.defender_id,
        defender_name: c.defender_name,
        mise: c.mise,
        status: c.status,
        winner_id: c.winner_id,
        attacker_roll: c.attacker_roll,
        defender_roll: c.defender_roll,
        chaos_event: c.chaos_event,
        special_attack: c.special_attack,
        defender_special: c.defender_special,
        coins_transferred: c.coins_transferred,
        result_message: c.result_message,
        message_id: c.message_id,
        created_at: c.created_at.to_rfc3339(),
        accepted_at: c.accepted_at.map(|d| d.to_rfc3339()),
        resolved_at: c.resolved_at.map(|d| d.to_rfc3339()),
    }
}

#[tonic::async_trait]
impl CoudeCombatsService for CoudeCombatsGrpc {
    async fn list(
        &self,
        request: Request<proto::ListCombatsRequest>,
    ) -> Result<Response<proto::CombatList>, Status> {
        let req = request.into_inner();
        let limit = if req.limit <= 0 { 50 } else { req.limit.min(500) };
        let list = self
            .uc
            .list(&req.guild_id, req.status.as_deref(), limit)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CombatList {
            combats: list.into_iter().map(combat_to_proto).collect(),
        }))
    }

    async fn get(
        &self,
        request: Request<proto::GetCombatRequest>,
    ) -> Result<Response<proto::CoudeCombat>, Status> {
        let id = parse_uuid(&request.into_inner().id)?;
        let c = self.uc.get(id).await.map_err(domain_to_status)?;
        Ok(Response::new(combat_to_proto(c)))
    }

    async fn get_pending_for_attacker(
        &self,
        request: Request<proto::GetPendingForAttackerRequest>,
    ) -> Result<Response<proto::MaybeCombat>, Status> {
        let req = request.into_inner();
        let c = self
            .uc
            .get_pending_for_attacker(&req.guild_id, &req.attacker_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::MaybeCombat {
            combat: c.map(combat_to_proto),
        }))
    }

    async fn get_pending_for_defender(
        &self,
        request: Request<proto::GetPendingForDefenderRequest>,
    ) -> Result<Response<proto::MaybeCombat>, Status> {
        let req = request.into_inner();
        let c = self
            .uc
            .get_pending_for_defender(&req.guild_id, &req.defender_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::MaybeCombat {
            combat: c.map(combat_to_proto),
        }))
    }

    async fn list_expired_pending(
        &self,
        _request: Request<proto::Empty>,
    ) -> Result<Response<proto::CombatList>, Status> {
        let list = self
            .uc
            .list_expired_pending()
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CombatList {
            combats: list.into_iter().map(combat_to_proto).collect(),
        }))
    }

    async fn get_betting_for_participant(
        &self,
        request: Request<proto::GetBettingForParticipantRequest>,
    ) -> Result<Response<proto::MaybeCombat>, Status> {
        let req = request.into_inner();
        let c = self
            .uc
            .get_betting_for_participant(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::MaybeCombat {
            combat: c.map(combat_to_proto),
        }))
    }

    async fn create(
        &self,
        request: Request<proto::CreateCombatRequest>,
    ) -> Result<Response<proto::CoudeCombat>, Status> {
        let req = request.into_inner();
        let c = self
            .uc
            .create(NewCoudeCombat {
                guild_id: req.guild_id,
                channel_id: req.channel_id,
                attacker_id: req.attacker_id,
                attacker_name: req.attacker_name,
                defender_id: req.defender_id,
                defender_name: req.defender_name,
                mise: req.mise,
                special_attack: req.special_attack,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(combat_to_proto(c)))
    }

    async fn cancel(
        &self,
        request: Request<proto::CancelCombatRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let id = parse_uuid(&request.into_inner().id)?;
        self.uc.cancel(id).await.map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn resolve(
        &self,
        request: Request<proto::ResolveCombatRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.id)?;
        self.uc
            .resolve(
                id,
                CombatResolution {
                    status: req.status,
                    winner_id: req.winner_id,
                    attacker_roll: req.attacker_roll,
                    defender_roll: req.defender_roll,
                    chaos_event: req.chaos_event,
                    result_message: req.result_message,
                    coins_transferred: req.coins_transferred,
                },
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn set_betting(
        &self,
        request: Request<proto::SetBettingRequest>,
    ) -> Result<Response<proto::SetBettingResponse>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.id)?;
        let transitioned = self
            .uc
            .set_betting(id, &req.message_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SetBettingResponse { transitioned }))
    }

    async fn expire(
        &self,
        request: Request<proto::ExpireCombatRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let id = parse_uuid(&request.into_inner().id)?;
        self.uc.expire(id).await.map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn set_defender_special(
        &self,
        request: Request<proto::SetDefenderSpecialRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.id)?;
        self.uc
            .set_defender_special(id, &req.item_key)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn resolve_betting_batch(
        &self,
        _request: Request<proto::Empty>,
    ) -> Result<Response<proto::ResolvedBettingBatch>, Status> {
        let out = self
            .resolve_batch_uc
            .resolve_batch()
            .await
            .map_err(domain_to_status)?;
        let combats = out
            .into_iter()
            .map(|c| proto::ResolvedBettingCombat {
                combat_id: c.combat_id,
                guild_id: c.guild_id,
                channel_id: c.channel_id,
                message_id: c.message_id,
                result_message: c.result_message,
                winner_id: c.winner_id,
                loser_id: c.loser_id,
                coins_transferred: c.coins_transferred,
                is_draw: c.is_draw,
                taunt_events: c.taunt_events.into_iter().map(taunt_event_to_proto).collect(),
            })
            .collect();
        Ok(Response::new(proto::ResolvedBettingBatch { combats }))
    }

    async fn expire_combats_batch(
        &self,
        _request: Request<proto::Empty>,
    ) -> Result<Response<proto::ExpiredCombatsBatch>, Status> {
        let out = self
            .expire_batch_uc
            .expire_batch()
            .await
            .map_err(domain_to_status)?;
        let combats = out
            .into_iter()
            .map(|c| proto::ExpiredCombat {
                combat_id: c.combat_id,
                guild_id: c.guild_id,
                channel_id: c.channel_id,
                defender_id: c.defender_id,
                defender_name: c.defender_name,
                penalty: c.penalty,
            })
            .collect();
        Ok(Response::new(proto::ExpiredCombatsBatch { combats }))
    }

    async fn resolve_combat_now(
        &self,
        request: Request<proto::ResolveCombatNowRequest>,
    ) -> Result<Response<proto::ResolvedCombatNowResponse>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.combat_id)?;
        let out = self
            .resolve_now_uc
            .resolve_now(id)
            .await
            .map_err(domain_to_status)?;
        let fields = out
            .fields
            .into_iter()
            .map(|f| proto::ResolvedCombatEmbedField {
                name: f.name,
                value: f.value,
                inline: f.inline,
            })
            .collect();
        Ok(Response::new(proto::ResolvedCombatNowResponse {
            combat_id: out.combat_id,
            title: out.title,
            description: out.description,
            color: out.color,
            fields,
            taunt_events: out
                .taunt_events
                .into_iter()
                .map(taunt_event_to_proto)
                .collect(),
            vendetta_humiliation: out.vendetta_humiliation.map(|h| {
                proto::VendettaHumiliation {
                    target_user_id: h.target_user_id,
                    challenger_user_id: h.challenger_user_id,
                }
            }),
        }))
    }
}


