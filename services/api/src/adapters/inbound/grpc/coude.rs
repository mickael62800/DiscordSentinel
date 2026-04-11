//! Implementation gRPC partielle du `CoudePlayerService` (Phase 7A).
//!
//! Wrappe `ManageCoudePlayersUseCase` — uniquement les 6 methodes du hot path
//! (CRUD joueur + progression + coins + HP). Les domaines combat/bets/economy/
//! inventory/social du Coup de Coude restent en HTTP.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use sentinel_proto::coude::v1 as proto;
use sentinel_proto::coude::v1::coude_player_service_server::CoudePlayerService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::{CoudePlayer, XpProgress};
use crate::domain::value_objects::CoudeClass;
use crate::ports::inbound::ManageCoudePlayersUseCase;

pub struct CoudePlayerGrpc {
    pub players_uc: Arc<dyn ManageCoudePlayersUseCase>,
}

#[tonic::async_trait]
impl CoudePlayerService for CoudePlayerGrpc {
    async fn get_or_create_player(
        &self,
        request: Request<proto::GetOrCreatePlayerRequest>,
    ) -> Result<Response<proto::CoudePlayer>, Status> {
        let req = request.into_inner();
        let player = self
            .players_uc
            .get_or_create(req.guild_id, req.user_id, req.username)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(coude_player_to_proto(player)))
    }

    async fn get_player(
        &self,
        request: Request<proto::GetPlayerRequest>,
    ) -> Result<Response<proto::CoudePlayer>, Status> {
        let req = request.into_inner();
        let player = self
            .players_uc
            .get(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(coude_player_to_proto(player)))
    }

    async fn update_player_class(
        &self,
        request: Request<proto::UpdatePlayerClassRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        // On valide la classe pour eviter d'envoyer une string invalide en BDD.
        if CoudeClass::from_str_lossy(&req.class).is_none() {
            return Err(Status::invalid_argument(format!(
                "Classe '{}' inconnue",
                req.class
            )));
        }
        self.players_uc
            .update_class(&req.guild_id, &req.user_id, &req.class)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn add_xp(
        &self,
        request: Request<proto::AddXpRequest>,
    ) -> Result<Response<proto::XpProgress>, Status> {
        let req = request.into_inner();
        let progress = self
            .players_uc
            .add_xp(&req.guild_id, &req.user_id, req.amount)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(xp_progress_to_proto(progress)))
    }

    async fn adjust_coins(
        &self,
        request: Request<proto::AdjustCoinsRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.players_uc
            .adjust_coins(&req.guild_id, &req.user_id, req.delta)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn update_hp(
        &self,
        request: Request<proto::UpdateHpRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.players_uc
            .update_hp(&req.guild_id, &req.user_id, req.hp_current, req.hp_max)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }
}

fn coude_player_to_proto(p: CoudePlayer) -> proto::CoudePlayer {
    proto::CoudePlayer {
        guild_id: p.guild_id,
        user_id: p.user_id,
        username: p.username,
        coins: p.coins,
        total_wins: p.total_wins,
        total_losses: p.total_losses,
        total_draws: p.total_draws,
        total_earned: p.total_earned,
        total_lost: p.total_lost,
        total_stolen: p.total_stolen,
        cowardice_count: p.cowardice_count,
        chaos_events: p.chaos_events,
        casino_wins: p.casino_wins,
        casino_losses: p.casino_losses,
        level: p.level,
        xp: p.xp,
        stat_points: p.stat_points,
        atk: p.atk,
        def: p.def,
        class: p.class.map(|c| c.as_str().to_string()),
        title: p.title,
        hp_current: p.hp_current,
        hp_max: p.hp_max,
        season: p.season,
        created_at: p.created_at.to_rfc3339(),
        updated_at: p.updated_at.to_rfc3339(),
    }
}

fn xp_progress_to_proto(x: XpProgress) -> proto::XpProgress {
    proto::XpProgress {
        new_xp: x.new_xp,
        new_level: x.new_level,
        leveled_up: x.leveled_up,
        stat_points_gained: x.stat_points_gained,
    }
}
