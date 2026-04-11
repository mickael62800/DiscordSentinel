//! Implementation gRPC complete du domaine Coup de Coude.
//!
//! Phase 7A : `CoudePlayerService` — 6 methodes hot path joueurs.
//! Phase 7A.opt F.1 : 5 services supplementaires wrappant les 5 use cases
//! restants (combats, bets, economy, inventory, social). coude-bot est
//! maintenant 100% gRPC pour ses appels metier.

use std::str::FromStr;
use std::sync::Arc;

use tonic::{Request, Response, Status};
use uuid::Uuid;

use sentinel_proto::coude::v1 as proto;
use sentinel_proto::coude::v1::coude_bets_service_server::CoudeBetsService;
use sentinel_proto::coude::v1::coude_combats_service_server::CoudeCombatsService;
use sentinel_proto::coude::v1::coude_economy_service_server::CoudeEconomyService;
use sentinel_proto::coude::v1::coude_inventory_service_server::CoudeInventoryService;
use sentinel_proto::coude::v1::coude_player_service_server::CoudePlayerService;
use sentinel_proto::coude::v1::coude_social_service_server::CoudeSocialService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::{
    BetPayout, BetResolutionPlan, CombatResolution, CoudeBet, CoudeCombat, CoudeCurrentSeason,
    CoudeEvent, CoudeFighterBetBonus, CoudeInsurance, CoudeInventoryItem, CoudeLeaderboardEntry,
    CoudePlayer, CoudePrime, LeaderboardCategory, NewCoudeBet, NewCoudeCombat, NewCoudePrime,
    NewDailyChaos, RefundSummary, XpProgress,
};
use crate::domain::value_objects::CoudeClass;
use crate::ports::inbound::{
    ManageCoudeBetsUseCase, ManageCoudeCombatsUseCase, ManageCoudeEconomyUseCase,
    ManageCoudeInventoryUseCase, ManageCoudePlayersUseCase, ManageCoudeSocialUseCase,
};

fn parse_uuid(s: &str) -> Result<Uuid, Status> {
    Uuid::from_str(s).map_err(|_| Status::invalid_argument(format!("UUID invalide: {s}")))
}

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

// ══════════════════════════════════════════════════════════════════════
// CoudeCombatsService (Phase 7A.opt F.1)
// ══════════════════════════════════════════════════════════════════════

pub struct CoudeCombatsGrpc {
    pub uc: Arc<dyn ManageCoudeCombatsUseCase>,
}

fn combat_to_proto(c: CoudeCombat) -> proto::CoudeCombat {
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
}

// ══════════════════════════════════════════════════════════════════════
// CoudeBetsService (Phase 7A.opt F.1)
// ══════════════════════════════════════════════════════════════════════

pub struct CoudeBetsGrpc {
    pub uc: Arc<dyn ManageCoudeBetsUseCase>,
}

fn bet_to_proto(b: CoudeBet) -> proto::CoudeBet {
    proto::CoudeBet {
        id: b.id,
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

fn bet_payout_to_proto(p: BetPayout) -> proto::BetPayout {
    proto::BetPayout {
        bet_id: p.bet_id,
        bettor_id: p.bettor_id,
        bettor_name: p.bettor_name,
        backed_id: p.backed_id,
        amount_bet: p.amount_bet,
        payout: p.payout,
        won: p.won,
    }
}

fn fighter_bonus_to_proto(b: CoudeFighterBetBonus) -> proto::FighterBetBonus {
    proto::FighterBetBonus {
        winner_id: b.winner_id,
        winner_bonus: b.winner_bonus,
        loser_id: b.loser_id,
        loser_bonus: b.loser_bonus,
        total_pot: b.total_pot,
    }
}

fn bet_resolution_plan_to_proto(p: BetResolutionPlan) -> proto::BetResolutionPlan {
    proto::BetResolutionPlan {
        payouts: p.payouts.into_iter().map(bet_payout_to_proto).collect(),
        fighter_bonus: p.fighter_bonus.map(fighter_bonus_to_proto),
    }
}

fn refund_summary_to_proto(s: RefundSummary) -> proto::RefundSummary {
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
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let combat_id = parse_uuid(&req.combat_id)?;
        self.uc
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
        Ok(Response::new(proto::Empty {}))
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
    ) -> Result<Response<proto::BetResolutionPlan>, Status> {
        let req = request.into_inner();
        let combat_id = parse_uuid(&req.combat_id)?;
        let plan = self
            .uc
            .resolve(combat_id, req.winner_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(bet_resolution_plan_to_proto(plan)))
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

// ══════════════════════════════════════════════════════════════════════
// CoudeEconomyService (Phase 7A.opt F.1)
// ══════════════════════════════════════════════════════════════════════

pub struct CoudeEconomyGrpc {
    pub uc: Arc<dyn ManageCoudeEconomyUseCase>,
}

#[tonic::async_trait]
impl CoudeEconomyService for CoudeEconomyGrpc {
    async fn transfer(
        &self,
        request: Request<proto::TransferRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .transfer(&req.guild_id, &req.from_id, &req.to_id, req.amount)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn steal(
        &self,
        request: Request<proto::StealRequest>,
    ) -> Result<Response<proto::StealResponse>, Status> {
        let req = request.into_inner();
        let stolen = self
            .uc
            .steal(&req.guild_id, &req.thief_id, &req.victim_id, req.amount)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::StealResponse { stolen }))
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

// ══════════════════════════════════════════════════════════════════════
// CoudeInventoryService (Phase 7A.opt F.1)
// ══════════════════════════════════════════════════════════════════════

pub struct CoudeInventoryGrpc {
    pub uc: Arc<dyn ManageCoudeInventoryUseCase>,
}

fn inventory_item_to_proto(i: CoudeInventoryItem) -> proto::CoudeInventoryItem {
    proto::CoudeInventoryItem {
        guild_id: i.guild_id,
        user_id: i.user_id,
        item_key: i.item_key,
        quantity: i.quantity,
    }
}

fn prime_to_proto(p: CoudePrime) -> proto::CoudePrime {
    proto::CoudePrime {
        id: p.id.to_string(),
        guild_id: p.guild_id,
        target_id: p.target_id,
        target_name: p.target_name,
        placed_by_id: p.placed_by_id,
        placed_by_name: p.placed_by_name,
        amount: p.amount,
        claimed: p.claimed,
        claimed_by_id: p.claimed_by_id,
        claimed_by_name: p.claimed_by_name,
        claimed_at: p.claimed_at.map(|d| d.to_rfc3339()),
        created_at: p.created_at.to_rfc3339(),
    }
}

fn insurance_to_proto(i: CoudeInsurance) -> proto::CoudeInsurance {
    proto::CoudeInsurance {
        id: i.id.to_string(),
        is_scam: i.is_scam,
        expires_at: i.expires_at.to_rfc3339(),
    }
}

#[tonic::async_trait]
impl CoudeInventoryService for CoudeInventoryGrpc {
    async fn list_inventory(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::InventoryList>, Status> {
        let req = request.into_inner();
        let items = self
            .uc
            .list_inventory(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::InventoryList {
            items: items.into_iter().map(inventory_item_to_proto).collect(),
        }))
    }

    async fn add_item(
        &self,
        request: Request<proto::AddItemRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .add_item(&req.guild_id, &req.user_id, &req.item_key)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn use_item(
        &self,
        request: Request<proto::UseItemRequest>,
    ) -> Result<Response<proto::UseItemResponse>, Status> {
        let req = request.into_inner();
        let consumed = self
            .uc
            .use_item(&req.guild_id, &req.user_id, &req.item_key)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::UseItemResponse { consumed }))
    }

    async fn has_item(
        &self,
        request: Request<proto::HasItemRequest>,
    ) -> Result<Response<proto::BoolValue>, Status> {
        let req = request.into_inner();
        let v = self
            .uc
            .has_item(&req.guild_id, &req.user_id, &req.item_key)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::BoolValue { value: v }))
    }

    async fn create_prime(
        &self,
        request: Request<proto::CreatePrimeRequest>,
    ) -> Result<Response<proto::CoudePrime>, Status> {
        let req = request.into_inner();
        let prime = self
            .uc
            .create_prime(NewCoudePrime {
                guild_id: req.guild_id,
                target_id: req.target_id,
                target_name: req.target_name,
                placed_by_id: req.placed_by_id,
                placed_by_name: req.placed_by_name,
                amount: req.amount,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(prime_to_proto(prime)))
    }

    async fn list_active_primes(
        &self,
        request: Request<proto::ListActivePrimesRequest>,
    ) -> Result<Response<proto::PrimeList>, Status> {
        let req = request.into_inner();
        let primes = self
            .uc
            .list_active_primes(&req.guild_id, &req.target_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::PrimeList {
            primes: primes.into_iter().map(prime_to_proto).collect(),
        }))
    }

    async fn claim_primes(
        &self,
        request: Request<proto::ClaimPrimesRequest>,
    ) -> Result<Response<proto::Int64Value>, Status> {
        let req = request.into_inner();
        let total = self
            .uc
            .claim_primes(&req.guild_id, &req.target_id, &req.claimer_id, &req.claimer_name)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Int64Value { value: total }))
    }

    async fn buy_insurance(
        &self,
        request: Request<proto::BuyInsuranceRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .buy_insurance(&req.guild_id, &req.user_id, req.is_scam)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn get_active_insurance(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::MaybeInsurance>, Status> {
        let req = request.into_inner();
        let ins = self
            .uc
            .get_active_insurance(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::MaybeInsurance {
            insurance: ins.map(insurance_to_proto),
        }))
    }

    async fn expire_insurance(
        &self,
        request: Request<proto::ExpireInsuranceRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let insurance_id = parse_uuid(&request.into_inner().insurance_id)?;
        self.uc
            .expire_insurance(insurance_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }
}

// ══════════════════════════════════════════════════════════════════════
// CoudeSocialService (Phase 7A.opt F.1)
// ══════════════════════════════════════════════════════════════════════

pub struct CoudeSocialGrpc {
    pub uc: Arc<dyn ManageCoudeSocialUseCase>,
}

fn proto_to_leaderboard_category(v: i32) -> LeaderboardCategory {
    match proto::LeaderboardCategory::try_from(v).unwrap_or(proto::LeaderboardCategory::Unspecified) {
        proto::LeaderboardCategory::Thieves => LeaderboardCategory::Thieves,
        proto::LeaderboardCategory::Cowards => LeaderboardCategory::Cowards,
        proto::LeaderboardCategory::Chaos => LeaderboardCategory::Chaos,
        proto::LeaderboardCategory::Level => LeaderboardCategory::Level,
        _ => LeaderboardCategory::Richest,
    }
}

fn leaderboard_entry_to_proto(e: CoudeLeaderboardEntry) -> proto::CoudeLeaderboardEntry {
    proto::CoudeLeaderboardEntry {
        user_id: e.user_id,
        username: e.username,
        value: e.value,
    }
}

fn event_to_proto(e: CoudeEvent) -> proto::CoudeEvent {
    proto::CoudeEvent {
        id: e.id.to_string(),
        guild_id: e.guild_id,
        active: e.active,
        expires_at: e.expires_at.to_rfc3339(),
        created_at: e.created_at.to_rfc3339(),
    }
}

fn current_season_to_proto(s: CoudeCurrentSeason) -> proto::CoudeCurrentSeason {
    proto::CoudeCurrentSeason {
        season_number: s.season_number,
        started_at: s.started_at.to_rfc3339(),
        ends_at: s.ends_at.to_rfc3339(),
        days_remaining: s.days_remaining,
    }
}

#[tonic::async_trait]
impl CoudeSocialService for CoudeSocialGrpc {
    async fn check_cooldown(
        &self,
        request: Request<proto::CheckCooldownRequest>,
    ) -> Result<Response<proto::CheckCooldownResponse>, Status> {
        let req = request.into_inner();
        let r = self
            .uc
            .check_cooldown(&req.guild_id, &req.user_id, &req.action)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CheckCooldownResponse {
            available_at: r.map(|d| d.to_rfc3339()),
        }))
    }

    async fn set_cooldown(
        &self,
        request: Request<proto::SetCooldownRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .set_cooldown(&req.guild_id, &req.user_id, &req.action, req.duration_secs)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn leaderboard(
        &self,
        request: Request<proto::LeaderboardRequest>,
    ) -> Result<Response<proto::LeaderboardList>, Status> {
        let req = request.into_inner();
        let cat = proto_to_leaderboard_category(req.category);
        let limit = if req.limit <= 0 { 10 } else { req.limit.min(100) };
        let list = self
            .uc
            .leaderboard(&req.guild_id, cat, limit)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::LeaderboardList {
            entries: list.into_iter().map(leaderboard_entry_to_proto).collect(),
        }))
    }

    async fn list_active_events(
        &self,
        request: Request<proto::ListActiveEventsRequest>,
    ) -> Result<Response<proto::EventList>, Status> {
        let req = request.into_inner();
        let list = self
            .uc
            .list_active_events(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::EventList {
            events: list.into_iter().map(event_to_proto).collect(),
        }))
    }

    async fn log_daily_chaos(
        &self,
        request: Request<proto::LogDailyChaosRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .log_daily_chaos(NewDailyChaos {
                guild_id: req.guild_id,
                loser_id: req.loser_id,
                loser_name: req.loser_name,
                winner_id: req.winner_id,
                winner_name: req.winner_name,
                amount: req.amount,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn current_season(
        &self,
        request: Request<proto::CurrentSeasonRequest>,
    ) -> Result<Response<proto::CoudeCurrentSeason>, Status> {
        let s = self
            .uc
            .current_season(&request.into_inner().guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(current_season_to_proto(s)))
    }
}
