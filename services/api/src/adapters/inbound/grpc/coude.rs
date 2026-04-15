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

    async fn hp_regen_tick(
        &self,
        request: Request<proto::HpRegenTickRequest>,
    ) -> Result<Response<proto::HpRegenTickResponse>, Status> {
        let req = request.into_inner();
        let updated = self
            .players_uc
            .regen_hp_tick(req.rate_0_25, req.rate_25_50, req.rate_50_75, req.rate_75_100)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::HpRegenTickResponse { updated }))
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
        // Fix bug /repos : champs indispensables aux checks cooldown cote bot.
        repos_last_used: p.repos_last_used.map(|d| d.to_rfc3339()),
        hp_last_regen: p.hp_last_regen.map(|d| d.to_rfc3339()),
        class_changed_at: p.class_changed_at.map(|d| d.to_rfc3339()),
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
    pub resolve_batch_uc: Arc<dyn crate::ports::inbound::ResolveBettingBatchUseCase>,
    pub expire_batch_uc: Arc<dyn crate::ports::inbound::ExpireCombatsBatchUseCase>,
    pub resolve_now_uc: Arc<dyn crate::ports::inbound::ResolveCombatNowUseCase>,
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
        }))
    }
}

fn taunt_event_to_proto(e: crate::domain::entities::TauntEvent) -> proto::TauntEvent {
    proto::TauntEvent {
        channel_id: e.channel_id,
        target_user_id: e.target_user_id,
        message: e.message,
        nickname_suffix: e.nickname_suffix,
        streak_kind: e.streak_kind.to_string(),
        streak_value: e.streak_value,
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
    pub steal_protections_uc: Arc<dyn crate::ports::inbound::ManageCoudeStealProtectionsUseCase>,
    pub steal_boosts_uc: Arc<dyn crate::ports::inbound::ManageCoudeStealBoostsUseCase>,
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
        let inserted = self.uc
            .buy_insurance(&req.guild_id, &req.user_id, req.is_scam, req.duration_seconds)
            .await
            .map_err(domain_to_status)?;
        if !inserted {
            return Err(Status::already_exists(
                "Une assurance active existe deja pour ce joueur",
            ));
        }
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

    // ── Phase 9 Part B : abonnements anti-vol ──

    async fn list_active_steal_protections(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::StealProtectionList>, Status> {
        let req = request.into_inner();
        let list = self
            .steal_protections_uc
            .list_active(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::StealProtectionList {
            protections: list.into_iter().map(steal_protection_to_proto).collect(),
        }))
    }

    async fn price_steal_protection(
        &self,
        request: Request<proto::PriceStealProtectionRequest>,
    ) -> Result<Response<proto::Int64Value>, Status> {
        let req = request.into_inner();
        let duration = proto_steal_duration_to_domain(req.duration)
            .ok_or_else(|| Status::invalid_argument("duree invalide"))?;
        let price = self
            .steal_protections_uc
            .price_for(&req.item_key, duration)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Int64Value { value: price }))
    }

    async fn buy_steal_protection(
        &self,
        request: Request<proto::BuyStealProtectionRequest>,
    ) -> Result<Response<proto::BuyStealProtectionResponse>, Status> {
        let req = request.into_inner();
        let duration = proto_steal_duration_to_domain(req.duration)
            .ok_or_else(|| Status::invalid_argument("duree invalide"))?;
        let cost = self
            .steal_protections_uc
            .price_for(&req.item_key, duration)
            .await
            .map_err(domain_to_status)?;
        let expires_at = self
            .steal_protections_uc
            .subscribe(&req.guild_id, &req.user_id, &req.item_key, duration)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::BuyStealProtectionResponse {
            expires_at: expires_at.to_rfc3339(),
            cost,
        }))
    }

    async fn try_trigger_steal_protection(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::MaybeStealProtectionTrigger>, Status> {
        let req = request.into_inner();
        let trigger = self
            .steal_protections_uc
            .try_trigger(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::MaybeStealProtectionTrigger {
            trigger: trigger.map(|t| proto::StealProtectionTrigger {
                item_key: t.item_key,
                item_name: t.item_name,
                rolled_value: t.rolled_value,
                block_chance_percent: t.block_chance_percent,
            }),
        }))
    }

    // ── Phase 9 Part C : boost voleur ──

    async fn list_active_steal_boosts(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::StealBoostList>, Status> {
        let req = request.into_inner();
        let list = self
            .steal_boosts_uc
            .list_active(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::StealBoostList {
            boosts: list.into_iter().map(steal_boost_to_proto).collect(),
        }))
    }

    async fn price_steal_boost(
        &self,
        request: Request<proto::PriceStealBoostRequest>,
    ) -> Result<Response<proto::Int64Value>, Status> {
        let req = request.into_inner();
        let duration = proto_steal_duration_to_domain(req.duration)
            .ok_or_else(|| Status::invalid_argument("duree invalide"))?;
        let price = self
            .steal_boosts_uc
            .price_for(&req.item_key, duration)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Int64Value { value: price }))
    }

    async fn buy_steal_boost(
        &self,
        request: Request<proto::BuyStealBoostRequest>,
    ) -> Result<Response<proto::BuyStealBoostResponse>, Status> {
        let req = request.into_inner();
        let duration = proto_steal_duration_to_domain(req.duration)
            .ok_or_else(|| Status::invalid_argument("duree invalide"))?;
        let cost = self
            .steal_boosts_uc
            .price_for(&req.item_key, duration)
            .await
            .map_err(domain_to_status)?;
        let expires_at = self
            .steal_boosts_uc
            .subscribe(&req.guild_id, &req.user_id, &req.item_key, duration)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::BuyStealBoostResponse {
            expires_at: expires_at.to_rfc3339(),
            cost,
        }))
    }

    async fn get_steal_boost_total(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::Int64Value>, Status> {
        let req = request.into_inner();
        let total = self
            .steal_boosts_uc
            .total_bonus(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Int64Value {
            value: total as i64,
        }))
    }
}

fn steal_boost_to_proto(b: crate::domain::entities::CoudeStealBoost) -> proto::CoudeStealBoost {
    proto::CoudeStealBoost {
        id: b.id.to_string(),
        guild_id: b.guild_id,
        user_id: b.user_id,
        item_key: b.item_key,
        expires_at: b.expires_at.to_rfc3339(),
        created_at: b.created_at.to_rfc3339(),
    }
}

fn steal_protection_to_proto(
    p: crate::domain::entities::CoudeStealProtection,
) -> proto::CoudeStealProtection {
    proto::CoudeStealProtection {
        id: p.id.to_string(),
        guild_id: p.guild_id,
        user_id: p.user_id,
        item_key: p.item_key,
        expires_at: p.expires_at.to_rfc3339(),
        created_at: p.created_at.to_rfc3339(),
    }
}

fn proto_steal_duration_to_domain(
    v: i32,
) -> Option<crate::domain::entities::StealProtectionDuration> {
    use crate::domain::entities::StealProtectionDuration as D;
    use proto::StealProtectionDurationKind as P;
    match P::try_from(v).ok()? {
        P::StealProtectionDurationUnspecified => None,
        P::StealProtectionDurationOneDay => Some(D::OneDay),
        P::StealProtectionDurationThreeDays => Some(D::ThreeDays),
        P::StealProtectionDurationFiveDays => Some(D::FiveDays),
        P::StealProtectionDurationSevenDays => Some(D::SevenDays),
    }
}

// ══════════════════════════════════════════════════════════════════════
// CoudeSocialService (Phase 7A.opt F.1)
// ══════════════════════════════════════════════════════════════════════

pub struct CoudeSocialGrpc {
    pub uc: Arc<dyn ManageCoudeSocialUseCase>,
    pub catalog_uc: Arc<dyn crate::ports::inbound::ManageCoudeCatalogUseCase>,
    pub cashbox_uc: Arc<dyn crate::ports::inbound::ManageCoudeCashboxUseCase>,
    pub taunts_uc: Arc<dyn crate::ports::inbound::ManageCoudeTauntsUseCase>,
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

    async fn get_catalog(
        &self,
        _request: Request<proto::Empty>,
    ) -> Result<Response<proto::CoudeCatalogResponse>, Status> {
        let cat = self
            .catalog_uc
            .get_catalog()
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CoudeCatalogResponse {
            classes: cat
                .classes
                .into_iter()
                .map(|c| proto::ClassInfo {
                    name: c.name,
                    emoji: c.emoji,
                    base_atk: c.base_atk,
                    base_def: c.base_def,
                    atk_growth: c.atk_growth,
                    def_growth: c.def_growth,
                    dodge_chance: c.dodge_chance,
                    steal_bonus: c.steal_bonus,
                    description: c.description,
                    passif_key: c.passif_key,
                    passif_description: c.passif_description,
                    passif_reveal: c.passif_reveal,
                })
                .collect(),
            shop_items: cat
                .shop_items
                .into_iter()
                .map(|i| proto::ShopItemInfo {
                    key: i.key,
                    name: i.name,
                    emoji: i.emoji,
                    price: i.price,
                    description: i.description,
                    category: i.category,
                    heal_amount: i.heal_amount,
                })
                .collect(),
            level_table: cat
                .level_table
                .into_iter()
                .map(|l| proto::LevelEntry {
                    level: l.level,
                    title: l.title,
                    xp_cumul: l.xp_cumul,
                })
                .collect(),
            matchmaking_buckets: cat
                .matchmaking_buckets
                .into_iter()
                .map(|b| proto::MatchmakingBucket {
                    gap_min: b.gap_min,
                    gap_max: b.gap_max,
                    handicap: b.handicap,
                    blocked: b.blocked,
                })
                .collect(),
            anti_theft_items: cat
                .anti_theft_items
                .into_iter()
                .map(|a| proto::AntiTheftItem {
                    key: a.key,
                    block_chance_percent: a.block_chance_percent,
                })
                .collect(),
            max_level: cat.max_level,
            hp_base: cat.hp_base,
            hp_per_def: cat.hp_per_def,
        }))
    }

    async fn get_cashbox(
        &self,
        request: Request<proto::GetCashboxRequest>,
    ) -> Result<Response<proto::CoudeCashboxState>, Status> {
        let guild_id = request.into_inner().guild_id;
        let cb = self
            .cashbox_uc
            .get_cashbox(&guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CoudeCashboxState {
            guild_id: cb.guild_id,
            balance: cb.balance,
            total_collected: cb.total_collected,
            total_redistributed: cb.total_redistributed,
            last_redistribution_at: cb.last_redistribution_at.map(|dt| dt.to_rfc3339()),
        }))
    }

    async fn redistribute_cashbox(
        &self,
        request: Request<proto::GetCashboxRequest>,
    ) -> Result<Response<proto::RedistributeCashboxResponse>, Status> {
        let guild_id = request.into_inner().guild_id;
        let outcome = self
            .cashbox_uc
            .redistribute_weekly(&guild_id)
            .await
            .map_err(domain_to_status)?;
        match outcome {
            None => Ok(Response::new(proto::RedistributeCashboxResponse {
                executed: false,
                redistribution_id: None,
                total_amount: 0,
                winners: vec![],
                guild_id,
            })),
            Some(o) => Ok(Response::new(redistribution_to_proto(guild_id, o))),
        }
    }

    async fn redistribute_due_cashboxes(
        &self,
        request: Request<proto::RedistributeDueRequest>,
    ) -> Result<Response<proto::RedistributeDueResponse>, Status> {
        let min_days = request.into_inner().min_days_since_last.max(0);
        let results = self
            .cashbox_uc
            .redistribute_due_guilds(min_days)
            .await
            .map_err(domain_to_status)?;
        let redistributed = results
            .into_iter()
            .map(|(guild_id, outcome)| redistribution_to_proto(guild_id, outcome))
            .collect();
        Ok(Response::new(proto::RedistributeDueResponse { redistributed }))
    }

    async fn deposit_cashbox(
        &self,
        request: Request<proto::DepositCashboxRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let source = proto_source_to_domain(req.source)
            .ok_or_else(|| Status::invalid_argument("source invalide"))?;
        self.cashbox_uc
            .deposit(&req.guild_id, req.amount, source)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    // ── Phase 9 Part D : railleries ──

    async fn track_steal_victim(
        &self,
        request: Request<proto::TrackStealVictimRequest>,
    ) -> Result<Response<proto::MaybeTauntEvent>, Status> {
        let req = request.into_inner();
        let ev = self
            .taunts_uc
            .on_player_stolen_from(&req.guild_id, &req.victim_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::MaybeTauntEvent {
            event: ev.map(taunt_event_to_proto),
        }))
    }

    async fn track_steal_defended(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.taunts_uc
            .on_player_defended_steal(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn get_taunts_config(
        &self,
        request: Request<proto::GetTauntsConfigRequest>,
    ) -> Result<Response<proto::TauntsConfigState>, Status> {
        let req = request.into_inner();
        let cfg = self
            .taunts_uc
            .get_config(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::TauntsConfigState {
            guild_id: cfg.guild_id,
            channel_id: cfg.channel_id,
            enabled: cfg.enabled,
        }))
    }

    async fn set_taunts_channel(
        &self,
        request: Request<proto::SetTauntsChannelRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.taunts_uc
            .set_channel(&req.guild_id, req.channel_id.as_deref())
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn set_taunts_enabled(
        &self,
        request: Request<proto::SetTauntsEnabledRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.taunts_uc
            .set_enabled(&req.guild_id, req.enabled)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn set_taunts_opt_out(
        &self,
        request: Request<proto::SetTauntsOptOutRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.taunts_uc
            .set_opt_out(&req.guild_id, &req.user_id, req.opted_out)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }
}

fn redistribution_to_proto(
    guild_id: String,
    outcome: crate::ports::inbound::RedistributionOutcome,
) -> proto::RedistributeCashboxResponse {
    proto::RedistributeCashboxResponse {
        executed: true,
        redistribution_id: Some(outcome.redistribution_id.to_string()),
        total_amount: outcome.total_amount,
        winners: outcome
            .winners
            .into_iter()
            .map(|(user_id, username, amount_won)| proto::CashboxWinner {
                user_id,
                username,
                amount_won,
            })
            .collect(),
        guild_id,
    }
}

fn proto_source_to_domain(
    source: i32,
) -> Option<crate::domain::entities::CashboxSource> {
    use crate::domain::entities::CashboxSource;
    use proto::CashboxDepositSource as P;
    match P::try_from(source).ok()? {
        P::CashboxSourceUnspecified => None,
        P::CashboxSourceShopPurchase => Some(CashboxSource::ShopPurchase),
        P::CashboxSourceInsurancePurchase => Some(CashboxSource::InsurancePurchase),
        P::CashboxSourceProtectionPurchase => Some(CashboxSource::ProtectionPurchase),
        P::CashboxSourceBoostPurchase => Some(CashboxSource::BoostPurchase),
        P::CashboxSourceClassChangeCost => Some(CashboxSource::ClassChangeCost),
        P::CashboxSourceResetStatsCost => Some(CashboxSource::ResetStatsCost),
        P::CashboxSourceDonationTax => Some(CashboxSource::DonationTax),
        P::CashboxSourceCowardicePenalty => Some(CashboxSource::CowardicePenalty),
        P::CashboxSourceBetCommission => Some(CashboxSource::BetCommission),
    }
}

// ══════════════════════════════════════════════════════════════════════
// Tests unitaires des converters proto <-> domain (Phase 7A.opt F.1)
// ══════════════════════════════════════════════════════════════════════
//
// Ces tests verifient que la traduction entre les entites de domaine et
// les messages protobuf est complete et correcte (aucun champ oublie ou
// melange). Ce sont des fonctions pures, donc pas de DB ni de mock.

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn ts() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 15, 12, 30, 0).unwrap()
    }

    #[test]
    fn parse_uuid_valid_ok() {
        let id = Uuid::new_v4();
        assert_eq!(parse_uuid(&id.to_string()).unwrap(), id);
    }

    #[test]
    fn parse_uuid_invalid_returns_invalid_argument() {
        let err = parse_uuid("not-a-uuid").unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        assert!(err.message().contains("UUID invalide"));
    }

    #[test]
    fn coude_player_to_proto_full_mapping() {
        let p = CoudePlayer {
            guild_id: "g1".into(),
            user_id: "u1".into(),
            username: "alice".into(),
            coins: 1234,
            total_wins: 5,
            total_losses: 3,
            total_draws: 1,
            total_earned: 2000,
            total_lost: 700,
            total_stolen: 50,
            cowardice_count: 2,
            chaos_events: 4,
            casino_wins: 7,
            casino_losses: 9,
            level: 12,
            xp: 4500,
            stat_points: 3,
            atk: 8,
            def: 6,
            class: Some(CoudeClass::Tank),
            title: Some("Champion".into()),
            class_changed_at: None,
            hp_current: 80,
            hp_max: 100,
            hp_last_regen: None,
            repos_last_used: None,
            season: 2,
            created_at: ts(),
            updated_at: ts(),
        };
        let pr = coude_player_to_proto(p.clone());
        assert_eq!(pr.guild_id, "g1");
        assert_eq!(pr.user_id, "u1");
        assert_eq!(pr.username, "alice");
        assert_eq!(pr.coins, 1234);
        assert_eq!(pr.total_wins, 5);
        assert_eq!(pr.total_losses, 3);
        assert_eq!(pr.total_draws, 1);
        assert_eq!(pr.level, 12);
        assert_eq!(pr.xp, 4500);
        assert_eq!(pr.atk, 8);
        assert_eq!(pr.def, 6);
        assert_eq!(pr.hp_current, 80);
        assert_eq!(pr.hp_max, 100);
        assert_eq!(pr.season, 2);
        assert_eq!(pr.class.as_deref(), Some(CoudeClass::Tank.as_str()));
        assert_eq!(pr.title.as_deref(), Some("Champion"));
        assert_eq!(pr.created_at, ts().to_rfc3339());
    }

    #[test]
    fn coude_player_to_proto_optional_class_none() {
        let p = CoudePlayer {
            guild_id: "g".into(), user_id: "u".into(), username: "x".into(),
            coins: 0, total_wins: 0, total_losses: 0, total_draws: 0,
            total_earned: 0, total_lost: 0, total_stolen: 0,
            cowardice_count: 0, chaos_events: 0, casino_wins: 0, casino_losses: 0,
            level: 1, xp: 0, stat_points: 0, atk: 0, def: 0,
            class: None, title: None, class_changed_at: None,
            hp_current: 100, hp_max: 100, hp_last_regen: None, repos_last_used: None,
            season: 1, created_at: ts(), updated_at: ts(),
        };
        let pr = coude_player_to_proto(p);
        assert!(pr.class.is_none());
        assert!(pr.title.is_none());
    }

    #[test]
    fn xp_progress_to_proto_mapping() {
        let x = XpProgress { new_xp: 1500, new_level: 8, leveled_up: true, stat_points_gained: 2 };
        let p = xp_progress_to_proto(x);
        assert_eq!(p.new_xp, 1500);
        assert_eq!(p.new_level, 8);
        assert!(p.leveled_up);
        assert_eq!(p.stat_points_gained, 2);
    }

    #[test]
    fn combat_to_proto_full_mapping() {
        let id = Uuid::new_v4();
        let c = CoudeCombat {
            id,
            guild_id: "g1".into(),
            channel_id: Some("c1".into()),
            attacker_id: "a".into(),
            attacker_name: "Atk".into(),
            defender_id: "d".into(),
            defender_name: "Def".into(),
            mise: 500,
            status: "resolved".into(),
            winner_id: Some("a".into()),
            attacker_roll: Some(15),
            defender_roll: Some(10),
            chaos_event: Some("eclipse".into()),
            special_attack: Some("uppercut".into()),
            defender_special: None,
            coins_transferred: Some(500),
            result_message: Some("Victoire".into()),
            message_id: Some("m1".into()),
            created_at: ts(),
            accepted_at: Some(ts()),
            resolved_at: Some(ts()),
        };
        let pr = combat_to_proto(c);
        assert_eq!(pr.id, id.to_string());
        assert_eq!(pr.channel_id.as_deref(), Some("c1"));
        assert_eq!(pr.mise, 500);
        assert_eq!(pr.status, "resolved");
        assert_eq!(pr.winner_id.as_deref(), Some("a"));
        assert_eq!(pr.attacker_roll, Some(15));
        assert_eq!(pr.coins_transferred, Some(500));
        assert_eq!(pr.accepted_at, Some(ts().to_rfc3339()));
    }

    #[test]
    fn bet_to_proto_mapping() {
        let b = CoudeBet {
            id: 42,
            guild_id: "g".into(),
            combat_id: Uuid::nil(),
            bettor_id: "u".into(),
            bettor_name: "Joe".into(),
            backed_id: "a".into(),
            amount: 100,
            won: Some(true),
            payout: Some(200),
        };
        let pr = bet_to_proto(b);
        assert_eq!(pr.id, 42);
        assert_eq!(pr.amount, 100);
        assert_eq!(pr.won, Some(true));
        assert_eq!(pr.payout, Some(200));
    }

    #[test]
    fn bet_payout_to_proto_mapping() {
        let p = BetPayout {
            bet_id: 1, bettor_id: "u".into(), bettor_name: "n".into(),
            backed_id: "a".into(), amount_bet: 100, payout: 250, won: true,
        };
        let pr = bet_payout_to_proto(p);
        assert_eq!(pr.bet_id, 1);
        assert_eq!(pr.amount_bet, 100);
        assert_eq!(pr.payout, 250);
        assert!(pr.won);
    }

    #[test]
    fn bet_resolution_plan_to_proto_with_bonus() {
        let plan = BetResolutionPlan {
            payouts: vec![],
            fighter_bonus: Some(CoudeFighterBetBonus {
                winner_id: "w".into(), winner_bonus: 1000,
                loser_id: "l".into(), loser_bonus: 500,
                total_pot: 2000,
            }),
        };
        let pr = bet_resolution_plan_to_proto(plan);
        assert!(pr.fighter_bonus.is_some());
        let b = pr.fighter_bonus.unwrap();
        assert_eq!(b.winner_bonus, 1000);
        assert_eq!(b.total_pot, 2000);
    }

    #[test]
    fn refund_summary_to_proto_mapping() {
        let s = RefundSummary { refunded_count: 3, refunded_total: 750 };
        let pr = refund_summary_to_proto(s);
        assert_eq!(pr.refunded_count, 3);
        assert_eq!(pr.refunded_total, 750);
    }

    #[test]
    fn inventory_item_to_proto_mapping() {
        let i = CoudeInventoryItem {
            guild_id: "g".into(), user_id: "u".into(),
            item_key: "potion".into(), quantity: 5,
        };
        let pr = inventory_item_to_proto(i);
        assert_eq!(pr.item_key, "potion");
        assert_eq!(pr.quantity, 5);
    }

    #[test]
    fn prime_to_proto_unclaimed() {
        let p = CoudePrime {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            target_id: "t".into(), target_name: "T".into(),
            placed_by_id: "p".into(), placed_by_name: "P".into(),
            amount: 1000, claimed: false,
            claimed_by_id: None, claimed_by_name: None, claimed_at: None,
            created_at: ts(),
        };
        let pr = prime_to_proto(p);
        assert_eq!(pr.amount, 1000);
        assert!(!pr.claimed);
        assert!(pr.claimed_by_id.is_none());
    }

    #[test]
    fn insurance_to_proto_mapping() {
        let id = Uuid::new_v4();
        let i = CoudeInsurance { id, is_scam: true, expires_at: ts() };
        let pr = insurance_to_proto(i);
        assert_eq!(pr.id, id.to_string());
        assert!(pr.is_scam);
        assert_eq!(pr.expires_at, ts().to_rfc3339());
    }

    #[test]
    fn proto_to_leaderboard_category_all_variants() {
        assert_eq!(
            proto_to_leaderboard_category(proto::LeaderboardCategory::Richest as i32),
            LeaderboardCategory::Richest
        );
        assert_eq!(
            proto_to_leaderboard_category(proto::LeaderboardCategory::Thieves as i32),
            LeaderboardCategory::Thieves
        );
        assert_eq!(
            proto_to_leaderboard_category(proto::LeaderboardCategory::Cowards as i32),
            LeaderboardCategory::Cowards
        );
        assert_eq!(
            proto_to_leaderboard_category(proto::LeaderboardCategory::Chaos as i32),
            LeaderboardCategory::Chaos
        );
        assert_eq!(
            proto_to_leaderboard_category(proto::LeaderboardCategory::Level as i32),
            LeaderboardCategory::Level
        );
        // Unspecified et valeur invalide => Richest (defaut)
        assert_eq!(
            proto_to_leaderboard_category(proto::LeaderboardCategory::Unspecified as i32),
            LeaderboardCategory::Richest
        );
        assert_eq!(
            proto_to_leaderboard_category(9999),
            LeaderboardCategory::Richest
        );
    }

    #[test]
    fn current_season_to_proto_mapping() {
        let s = CoudeCurrentSeason {
            season_number: 3, started_at: ts(), ends_at: ts(), days_remaining: 42,
        };
        let pr = current_season_to_proto(s);
        assert_eq!(pr.season_number, 3);
        assert_eq!(pr.days_remaining, 42);
    }

    #[test]
    fn leaderboard_entry_to_proto_mapping() {
        let e = CoudeLeaderboardEntry {
            user_id: "u".into(), username: "Top".into(), value: 9999,
        };
        let pr = leaderboard_entry_to_proto(e);
        assert_eq!(pr.user_id, "u");
        assert_eq!(pr.value, 9999);
    }

    #[test]
    fn event_to_proto_mapping() {
        let id = Uuid::new_v4();
        let e = CoudeEvent {
            id, guild_id: "g".into(), event_type: "happy_hour".into(), active: true,
            expires_at: ts(), created_at: ts(),
        };
        let pr = event_to_proto(e);
        assert_eq!(pr.id, id.to_string());
        assert!(pr.active);
    }
}
