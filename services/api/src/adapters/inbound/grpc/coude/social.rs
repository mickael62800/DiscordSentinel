//! Handler gRPC du service `CoudeSocialService`.
//!
//! Couvre toutes les fonctionnalites transversales :
//! - Cooldowns generiques + leaderboards + events + daily chaos + seasons
//! - Phase 8 : `GetCatalog` (classes, shop, matchmaking, progression)
//! - Phase 9 Part A : cashbox (get, deposit, redistribute, due)
//! - Phase 9 Part D : taunts (track/config)
//! - Phase 10 : braquage (attempt/cooldown/prison)
//!
//! Les helpers de mapping (leaderboard_entry_to_proto, event_to_proto,
//! current_season_to_proto, redistribution_to_proto,
//! proto_source_to_domain) sont locaux a ce fichier.

use std::sync::Arc;

use tonic::Request;
use tonic::Response;
use tonic::Status;
use sentinel_proto::coude::v1 as proto;
use sentinel_proto::coude::v1::coude_social_service_server::CoudeSocialService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::coude::social::Season;
use crate::domain::entities::coude::social::Event;
use crate::domain::entities::coude::social::LeaderboardEntry;
use crate::domain::entities::coude::social::LeaderboardCategory;
use crate::domain::entities::coude::social::NewDailyChaos;
use crate::ports::inbound::coude::manage_social::ManageCoudeSocialUseCase;

use super::taunt_event_to_proto;

pub struct SocialGrpc {
    pub uc: Arc<dyn ManageCoudeSocialUseCase>,
    pub catalog_uc: Arc<dyn crate::ports::inbound::coude::manage_catalog::ManageCoudeCatalogUseCase>,
    pub cashbox_uc: Arc<dyn crate::ports::inbound::coude::manage_cashbox::ManageCoudeCashboxUseCase>,
    pub taunts_uc: Arc<dyn crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase>,
    pub heist_uc: Arc<dyn crate::ports::inbound::coude::manage_heist::ManageCoudeHeistUseCase>,
}

pub(super) fn proto_to_leaderboard_category(v: i32) -> LeaderboardCategory {
    match proto::LeaderboardCategory::try_from(v).unwrap_or(proto::LeaderboardCategory::Unspecified) {
        proto::LeaderboardCategory::Thieves => LeaderboardCategory::Thieves,
        proto::LeaderboardCategory::Cowards => LeaderboardCategory::Cowards,
        proto::LeaderboardCategory::Chaos => LeaderboardCategory::Chaos,
        proto::LeaderboardCategory::Level => LeaderboardCategory::Level,
        _ => LeaderboardCategory::Richest,
    }
}

pub(super) fn leaderboard_entry_to_proto(e: LeaderboardEntry) -> proto::LeaderboardEntry {
    proto::LeaderboardEntry {
        user_id: e.user_id.into(),
        username: e.username,
        value: e.value,
    }
}

pub(super) fn event_to_proto(e: Event) -> proto::Event {
    proto::Event {
        id: e.id.to_string(),
        guild_id: e.guild_id,
        active: e.active,
        expires_at: e.expires_at.to_rfc3339(),
        created_at: e.created_at.to_rfc3339(),
    }
}

pub(super) fn current_season_to_proto(s: Season) -> proto::Season {
    proto::Season {
        season_number: s.season_number,
        started_at: s.started_at.to_rfc3339(),
        ends_at: s.ends_at.to_rfc3339(),
        days_remaining: s.days_remaining,
    }
}

#[tonic::async_trait]
impl CoudeSocialService for SocialGrpc {
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
    ) -> Result<Response<proto::Season>, Status> {
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
    ) -> Result<Response<proto::CatalogResponse>, Status> {
        let cat = self
            .catalog_uc
            .get_catalog()
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CatalogResponse {
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
    ) -> Result<Response<proto::CashboxState>, Status> {
        let guild_id = request.into_inner().guild_id;
        let cb = self
            .cashbox_uc
            .get_cashbox(&guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CashboxState {
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

    // ── Phase 10 : braquage ──

    async fn attempt_heist(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::HeistResult>, Status> {
        let req = request.into_inner();
        let outcome = self
            .heist_uc
            .attempt_heist(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::HeistResult {
            success: outcome.success,
            chance_percent: outcome.chance_percent,
            cashbox_total_before: outcome.cashbox_total_before,
            amount_stolen: outcome.amount_stolen,
            tools_consumed: outcome.tools_consumed,
            prison_released_at: outcome.prison_released_at.map(|d| d.to_rfc3339()),
        }))
    }

    async fn get_heist_cooldown(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::HeistCooldownState>, Status> {
        let req = request.into_inner();
        let status = self
            .heist_uc
            .get_cooldown_status(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::HeistCooldownState {
            ready: status.ready,
            next_attempt_at: status.next_attempt_at.map(|d| d.to_rfc3339()),
            last_success: status.last_success,
        }))
    }

    async fn get_prison_status(
        &self,
        request: Request<proto::UserInGuildRequest>,
    ) -> Result<Response<proto::PrisonStatusState>, Status> {
        let req = request.into_inner();
        let status = self
            .heist_uc
            .get_prison_status(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::PrisonStatusState {
            in_prison: status.in_prison,
            released_at: status.released_at.map(|d| d.to_rfc3339()),
            reason: status.reason,
        }))
    }

    async fn trigger_daily_chaos(
        &self,
        request: Request<proto::TriggerDailyChaosRequest>,
    ) -> Result<Response<proto::DailyChaosResponse>, Status> {
        let guild_id = request.into_inner().guild_id;
        let outcome = self
            .uc
            .trigger_daily_chaos(&guild_id)
            .await
            .map_err(domain_to_status)?;
        match outcome {
            Some(o) => Ok(Response::new(proto::DailyChaosResponse {
                triggered: true,
                loser_id: o.loser_id,
                loser_name: o.loser_name,
                winner_id: o.winner_id,
                winner_name: o.winner_name,
                amount: o.amount,
                channel_id: o.channel_id.into(),
                taunt_events: o
                    .taunt_events
                    .into_iter()
                    .map(super::taunt_event_to_proto)
                    .collect(),
            })),
            None => Ok(Response::new(proto::DailyChaosResponse {
                triggered: false,
                ..Default::default()
            })),
        }
    }
}

pub(super) fn redistribution_to_proto(
    guild_id: String,
    outcome: crate::ports::inbound::coude::manage_cashbox::RedistributionOutcome,
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

#[cfg(test)]
#[path = "tests/social.rs"]
mod tests;

pub(super) fn proto_source_to_domain(
    source: i32,
) -> Option<crate::domain::entities::coude::cashbox::CashboxSource> {
    use crate::domain::entities::coude::cashbox::CashboxSource;
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
