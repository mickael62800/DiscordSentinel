use super::*;
use async_trait::async_trait;
use chrono::Duration;
use chrono::Utc;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::domain::entities::coude::cashbox::CashboxSource;
use crate::domain::entities::coude::cashbox::Cashbox;
use crate::domain::entities::coude::social::CoudeCurrentSeason;
use crate::domain::entities::coude::social::CoudeEvent;
use crate::domain::entities::coude::social::CoudeLeaderboardEntry;
use crate::domain::entities::coude::taunt::TauntsConfig;
use crate::domain::entities::coude::social::DailyChaosOutcome;
use crate::domain::entities::coude::heist::HeistOutcome;
use crate::domain::entities::coude::social::LeaderboardCategory;
use crate::domain::entities::coude::social::NewDailyChaos;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_catalog::AntiTheftItemInfo;
use crate::ports::inbound::coude::manage_catalog::ClassInfo;
use crate::ports::inbound::coude::manage_catalog::Catalog;
use crate::ports::inbound::coude::manage_catalog::LevelEntry;
use crate::ports::inbound::coude::manage_catalog::MatchmakingBucket;
use crate::ports::inbound::coude::manage_catalog::ShopItemInfo;
use crate::ports::inbound::coude::manage_cashbox::RedistributionOutcome;
use crate::ports::inbound::coude::manage_heist::HeistCooldownStatus;
use crate::ports::inbound::coude::manage_heist::PrisonStatusInfo;
use crate::ports::inbound::coude::manage_cashbox::ManageCoudeCashboxUseCase;
use crate::ports::inbound::coude::manage_catalog::ManageCoudeCatalogUseCase;
use crate::ports::inbound::coude::manage_heist::ManageCoudeHeistUseCase;
use crate::ports::inbound::coude::manage_social::ManageCoudeSocialUseCase;
use crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
// ── Mocks ──

#[derive(Default)]
struct MockSocialUc {
    check_cooldown_return: Mutex<Option<chrono::DateTime<Utc>>>,
    set_cooldown_calls: Mutex<Vec<(String, String, String, i64)>>,
    leaderboard_calls: Mutex<Vec<(String, LeaderboardCategory, i64)>>,
    leaderboard_return: Mutex<Vec<CoudeLeaderboardEntry>>,
    events_return: Mutex<Vec<CoudeEvent>>,
    log_chaos_calls: Mutex<Vec<NewDailyChaos>>,
    chaos_return: Mutex<Option<DailyChaosOutcome>>,
    season_return: Mutex<Option<CoudeCurrentSeason>>,
}

#[async_trait]
impl ManageCoudeSocialUseCase for MockSocialUc {
    async fn check_cooldown(&self, _: &str, _: &str, _: &str) -> Result<Option<chrono::DateTime<Utc>>, DomainError> {
        Ok(*self.check_cooldown_return.lock().unwrap())
    }
    async fn set_cooldown(&self, g: &str, u: &str, a: &str, d: i64) -> Result<(), DomainError> {
        self.set_cooldown_calls.lock().unwrap().push((g.into(), u.into(), a.into(), d));
        Ok(())
    }
    async fn leaderboard(&self, g: &str, c: LeaderboardCategory, l: i64) -> Result<Vec<CoudeLeaderboardEntry>, DomainError> {
        self.leaderboard_calls.lock().unwrap().push((g.into(), c, l));
        Ok(self.leaderboard_return.lock().unwrap().clone())
    }
    async fn list_active_events(&self, _: &str) -> Result<Vec<CoudeEvent>, DomainError> {
        Ok(self.events_return.lock().unwrap().clone())
    }
    async fn log_daily_chaos(&self, c: NewDailyChaos) -> Result<(), DomainError> {
        self.log_chaos_calls.lock().unwrap().push(c);
        Ok(())
    }
    async fn trigger_daily_chaos(&self, _: &str) -> Result<Option<DailyChaosOutcome>, DomainError> {
        Ok(self.chaos_return.lock().unwrap().clone())
    }
    async fn current_season(&self, _: &str) -> Result<CoudeCurrentSeason, DomainError> {
        Ok(self.season_return.lock().unwrap().clone().unwrap_or(CoudeCurrentSeason {
            season_number: 1,
            started_at: Utc::now(),
            ends_at: Utc::now(),
            days_remaining: 0,
        }))
    }
}

#[derive(Default)]
struct MockCatalogUc {
    catalog: Mutex<Option<Catalog>>,
}

#[async_trait]
impl ManageCoudeCatalogUseCase for MockCatalogUc {
    async fn get_catalog(&self) -> Result<Catalog, DomainError> {
        Ok(self.catalog.lock().unwrap().clone().unwrap_or(Catalog {
            classes: vec![],
            shop_items: vec![],
            level_table: vec![],
            matchmaking_buckets: vec![],
            anti_theft_items: vec![],
            max_level: 0,
            hp_base: 0,
            hp_per_def: 0,
        }))
    }
}

#[derive(Default)]
struct MockCashboxUc {
    cashbox: Mutex<Option<Cashbox>>,
    redistribute_return: Mutex<Option<RedistributionOutcome>>,
    due_return: Mutex<Vec<(String, RedistributionOutcome)>>,
    deposit_calls: Mutex<Vec<(String, i64, CashboxSource)>>,
}

#[async_trait]
impl ManageCoudeCashboxUseCase for MockCashboxUc {
    async fn get_cashbox(&self, g: &str) -> Result<Cashbox, DomainError> {
        Ok(self.cashbox.lock().unwrap().clone().unwrap_or(Cashbox {
            guild_id: g.into(),
            balance: 0,
            total_collected: 0,
            total_redistributed: 0,
            last_redistribution_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }))
    }
    async fn deposit(&self, g: &str, a: i64, s: CashboxSource) -> Result<(), DomainError> {
        self.deposit_calls.lock().unwrap().push((g.into(), a, s));
        Ok(())
    }
    async fn redistribute_weekly(&self, _: &str) -> Result<Option<RedistributionOutcome>, DomainError> {
        Ok(self.redistribute_return.lock().unwrap().clone())
    }
    async fn redistribute_due_guilds(&self, _: i64) -> Result<Vec<(String, RedistributionOutcome)>, DomainError> {
        Ok(self.due_return.lock().unwrap().clone())
    }
    async fn list_redistributions(&self, _: &str, _: i64) -> Result<Vec<crate::domain::entities::coude::cashbox::CashboxRedistribution>, DomainError> {
        unimplemented!()
    }
    async fn list_entries(&self, _: Uuid) -> Result<Vec<crate::domain::entities::coude::cashbox::CashboxRedistributionEntry>, DomainError> {
        unimplemented!()
    }
}

#[derive(Default)]
struct MockTauntsUc {
    steal_victim_return: Mutex<Option<TauntEvent>>,
    defended_calls: Mutex<Vec<(String, String)>>,
    config: Mutex<Option<TauntsConfig>>,
    set_channel_calls: Mutex<Vec<(String, Option<String>)>>,
    set_enabled_calls: Mutex<Vec<(String, bool)>>,
    set_opt_out_calls: Mutex<Vec<(String, String, bool)>>,
}

#[async_trait]
impl ManageCoudeTauntsUseCase for MockTauntsUc {
    async fn on_player_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_player_lost(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_player_drew(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn on_player_stolen_from(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(self.steal_victim_return.lock().unwrap().clone())
    }
    async fn on_player_defended_steal(&self, g: &str, u: &str) -> Result<(), DomainError> {
        self.defended_calls.lock().unwrap().push((g.into(), u.into()));
        Ok(())
    }
    async fn on_bj_natural(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_bj_hand_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_bj_hand_bust(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_bankruptcy(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_jackpot(&self, _: &str, _: &str, _: i64) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_generous_donor(&self, _: &str, _: &str, _: i64) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn get_config(&self, g: &str) -> Result<TauntsConfig, DomainError> {
        Ok(self.config.lock().unwrap().clone().unwrap_or(TauntsConfig {
            guild_id: g.into(),
            channel_id: None,
            enabled: true,
            rename_enabled: true,
            messages_enabled: true,
        }))
    }
    async fn set_channel(&self, g: &str, c: Option<&str>) -> Result<(), DomainError> {
        self.set_channel_calls.lock().unwrap().push((g.into(), c.map(|s| s.to_string())));
        Ok(())
    }
    async fn set_enabled(&self, g: &str, e: bool) -> Result<(), DomainError> {
        self.set_enabled_calls.lock().unwrap().push((g.into(), e));
        Ok(())
    }
    async fn set_rename_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { unimplemented!() }
    async fn set_messages_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { unimplemented!() }
    async fn set_opt_out(&self, g: &str, u: &str, o: bool) -> Result<(), DomainError> {
        self.set_opt_out_calls.lock().unwrap().push((g.into(), u.into(), o));
        Ok(())
    }
    async fn is_opted_out(&self, _: &str, _: &str) -> Result<bool, DomainError> { unimplemented!() }
    async fn list_opt_outs(&self, _: &str) -> Result<Vec<String>, DomainError> { unimplemented!() }
}

#[derive(Default)]
struct MockHeistUc {
    cooldown: Mutex<Option<HeistCooldownStatus>>,
    prison: Mutex<Option<PrisonStatusInfo>>,
    outcome: Mutex<Option<HeistOutcome>>,
}

#[async_trait]
impl ManageCoudeHeistUseCase for MockHeistUc {
    async fn get_cooldown_status(&self, _: &str, _: &str) -> Result<HeistCooldownStatus, DomainError> {
        Ok(self.cooldown.lock().unwrap().clone().unwrap_or(HeistCooldownStatus {
            ready: true, next_attempt_at: None, last_success: None,
        }))
    }
    async fn get_prison_status(&self, _: &str, _: &str) -> Result<PrisonStatusInfo, DomainError> {
        Ok(self.prison.lock().unwrap().clone().unwrap_or(PrisonStatusInfo {
            in_prison: false, released_at: None, reason: None,
        }))
    }
    async fn attempt_heist(&self, _: &str, _: &str) -> Result<HeistOutcome, DomainError> {
        Ok(self.outcome.lock().unwrap().clone().unwrap_or(HeistOutcome {
            success: false, chance_percent: 0, cashbox_total_before: 0,
            amount_stolen: 0, tools_consumed: vec![], prison_released_at: None,
        }))
    }
}

fn mk(
    s: Arc<MockSocialUc>, c: Arc<MockCatalogUc>, cb: Arc<MockCashboxUc>,
    t: Arc<MockTauntsUc>, h: Arc<MockHeistUc>,
) -> CoudeSocialGrpc {
    CoudeSocialGrpc { uc: s, catalog_uc: c, cashbox_uc: cb, taunts_uc: t, heist_uc: h }
}

fn defaults() -> (Arc<MockSocialUc>, Arc<MockCatalogUc>, Arc<MockCashboxUc>, Arc<MockTauntsUc>, Arc<MockHeistUc>) {
    (Arc::new(MockSocialUc::default()), Arc::new(MockCatalogUc::default()),
     Arc::new(MockCashboxUc::default()), Arc::new(MockTauntsUc::default()),
     Arc::new(MockHeistUc::default()))
}

// ── proto_to_leaderboard_category ──

#[test]
fn category_mapping() {
    use proto::LeaderboardCategory as P;
    assert!(matches!(proto_to_leaderboard_category(P::Thieves as i32), LeaderboardCategory::Thieves));
    assert!(matches!(proto_to_leaderboard_category(P::Cowards as i32), LeaderboardCategory::Cowards));
    assert!(matches!(proto_to_leaderboard_category(P::Chaos as i32), LeaderboardCategory::Chaos));
    assert!(matches!(proto_to_leaderboard_category(P::Level as i32), LeaderboardCategory::Level));
    assert!(matches!(proto_to_leaderboard_category(P::Unspecified as i32), LeaderboardCategory::Richest));
    assert!(matches!(proto_to_leaderboard_category(99999), LeaderboardCategory::Richest));
}

// ── check_cooldown ──

#[tokio::test]
async fn check_cooldown_none_returns_none() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s, c, cb, t, h);
    let resp = g.check_cooldown(Request::new(proto::CheckCooldownRequest {
        guild_id: "g".into(), user_id: "u".into(), action: "daily".into(),
    })).await.unwrap();
    assert!(resp.into_inner().available_at.is_none());
}

#[tokio::test]
async fn check_cooldown_some_returns_rfc3339() {
    let (s, c, cb, t, h) = defaults();
    let dt = Utc::now() + Duration::hours(1);
    *s.check_cooldown_return.lock().unwrap() = Some(dt);
    let g = mk(s, c, cb, t, h);
    let resp = g.check_cooldown(Request::new(proto::CheckCooldownRequest {
        guild_id: "g".into(), user_id: "u".into(), action: "daily".into(),
    })).await.unwrap();
    assert_eq!(resp.into_inner().available_at, Some(dt.to_rfc3339()));
}

// ── set_cooldown ──

#[tokio::test]
async fn set_cooldown_delegates() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s.clone(), c, cb, t, h);
    g.set_cooldown(Request::new(proto::SetCooldownRequest {
        guild_id: "g".into(), user_id: "u".into(), action: "daily".into(), duration_secs: 3600,
    })).await.unwrap();
    let calls = s.set_cooldown_calls.lock().unwrap();
    assert_eq!(calls[0], ("g".into(), "u".into(), "daily".into(), 3600));
}

// ── leaderboard ──

#[tokio::test]
async fn leaderboard_default_limit_when_zero_or_negative() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s.clone(), c, cb, t, h);
    g.leaderboard(Request::new(proto::LeaderboardRequest {
        guild_id: "g".into(), category: 0, limit: 0,
    })).await.unwrap();
    assert_eq!(s.leaderboard_calls.lock().unwrap()[0].2, 10);
    g.leaderboard(Request::new(proto::LeaderboardRequest {
        guild_id: "g".into(), category: 0, limit: -5,
    })).await.unwrap();
    assert_eq!(s.leaderboard_calls.lock().unwrap()[1].2, 10);
}

#[tokio::test]
async fn leaderboard_caps_limit_at_100() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s.clone(), c, cb, t, h);
    g.leaderboard(Request::new(proto::LeaderboardRequest {
        guild_id: "g".into(), category: 0, limit: 5000,
    })).await.unwrap();
    assert_eq!(s.leaderboard_calls.lock().unwrap()[0].2, 100);
}

#[tokio::test]
async fn leaderboard_maps_entries() {
    let (s, c, cb, t, h) = defaults();
    s.leaderboard_return.lock().unwrap().push(CoudeLeaderboardEntry {
        user_id: "u1".into(), username: "Alice".into(), value: 42,
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.leaderboard(Request::new(proto::LeaderboardRequest {
        guild_id: "g".into(), category: 0, limit: 10,
    })).await.unwrap();
    let entries = resp.into_inner().entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].user_id, "u1");
    assert_eq!(entries[0].value, 42);
}

// ── list_active_events ──

#[tokio::test]
async fn list_active_events_maps() {
    let (s, c, cb, t, h) = defaults();
    s.events_return.lock().unwrap().push(CoudeEvent {
        id: Uuid::new_v4(), guild_id: "g".into(), event_type: "x".into(),
        active: true, expires_at: Utc::now(), created_at: Utc::now(),
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.list_active_events(Request::new(proto::ListActiveEventsRequest {
        guild_id: "g".into(),
    })).await.unwrap();
    assert_eq!(resp.into_inner().events.len(), 1);
}

// ── log_daily_chaos ──

#[tokio::test]
async fn log_daily_chaos_delegates_fields() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s.clone(), c, cb, t, h);
    g.log_daily_chaos(Request::new(proto::LogDailyChaosRequest {
        guild_id: "g".into(),
        loser_id: "l".into(), loser_name: "L".into(),
        winner_id: "w".into(), winner_name: "W".into(),
        amount: 100,
    })).await.unwrap();
    let calls = s.log_chaos_calls.lock().unwrap();
    assert_eq!(calls[0].guild_id, "g");
    assert_eq!(calls[0].loser_id, "l");
    assert_eq!(calls[0].winner_name, "W");
    assert_eq!(calls[0].amount, 100);
}

// ── current_season ──

#[tokio::test]
async fn current_season_maps() {
    let (s, c, cb, t, h) = defaults();
    *s.season_return.lock().unwrap() = Some(CoudeCurrentSeason {
        season_number: 3, started_at: Utc::now(), ends_at: Utc::now(), days_remaining: 42,
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.current_season(Request::new(proto::CurrentSeasonRequest {
        guild_id: "g".into(),
    })).await.unwrap();
    let s = resp.into_inner();
    assert_eq!(s.season_number, 3);
    assert_eq!(s.days_remaining, 42);
}

// ── get_catalog ──

#[tokio::test]
async fn get_catalog_maps_all_collections() {
    let (s, c, cb, t, h) = defaults();
    *c.catalog.lock().unwrap() = Some(Catalog {
        classes: vec![ClassInfo {
            name: "Guerrier".into(), emoji: "x".into(), base_atk: 10, base_def: 5,
            atk_growth: 2, def_growth: 1, dodge_chance: 0.1, steal_bonus: 0.2,
            description: "d".into(), passif_key: "pk".into(),
            passif_description: "pd".into(), passif_reveal: "pr".into(),
        }],
        shop_items: vec![ShopItemInfo {
            key: "potion".into(), name: "Potion".into(), emoji: "🧪".into(),
            price: 100, description: "heal".into(), category: "consumable".into(),
            heal_amount: 50,
        }],
        level_table: vec![LevelEntry { level: 1, title: "Noob".into(), xp_cumul: 0 }],
        matchmaking_buckets: vec![MatchmakingBucket {
            gap_min: 0, gap_max: 5, handicap: 1.0, blocked: false,
        }],
        anti_theft_items: vec![AntiTheftItemInfo { key: "lock".into(), block_chance_percent: 20 }],
        max_level: 50, hp_base: 100, hp_per_def: 10,
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.get_catalog(Request::new(proto::Empty {})).await.unwrap().into_inner();
    assert_eq!(resp.classes.len(), 1);
    assert_eq!(resp.classes[0].name, "Guerrier");
    assert_eq!(resp.shop_items[0].heal_amount, 50);
    assert_eq!(resp.level_table[0].title, "Noob");
    assert_eq!(resp.matchmaking_buckets[0].gap_max, 5);
    assert_eq!(resp.anti_theft_items[0].block_chance_percent, 20);
    assert_eq!(resp.max_level, 50);
    assert_eq!(resp.hp_base, 100);
    assert_eq!(resp.hp_per_def, 10);
}

// ── get_cashbox ──

#[tokio::test]
async fn get_cashbox_maps() {
    let (s, c, cb, t, h) = defaults();
    *cb.cashbox.lock().unwrap() = Some(Cashbox {
        guild_id: "g".into(), balance: 500, total_collected: 1000, total_redistributed: 500,
        last_redistribution_at: None, created_at: Utc::now(), updated_at: Utc::now(),
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.get_cashbox(Request::new(proto::GetCashboxRequest { guild_id: "g".into() }))
        .await.unwrap().into_inner();
    assert_eq!(resp.balance, 500);
    assert_eq!(resp.total_collected, 1000);
    assert!(resp.last_redistribution_at.is_none());
}

// ── redistribute_cashbox ──

#[tokio::test]
async fn redistribute_cashbox_none_returns_not_executed() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s, c, cb, t, h);
    let resp = g.redistribute_cashbox(Request::new(proto::GetCashboxRequest { guild_id: "g".into() }))
        .await.unwrap().into_inner();
    assert!(!resp.executed);
    assert_eq!(resp.total_amount, 0);
    assert_eq!(resp.guild_id, "g");
}

#[tokio::test]
async fn redistribute_cashbox_some_returns_winners() {
    let (s, c, cb, t, h) = defaults();
    let rid = Uuid::new_v4();
    *cb.redistribute_return.lock().unwrap() = Some(RedistributionOutcome {
        redistribution_id: rid, total_amount: 1000,
        winners: vec![("u1".into(), "Alice".into(), 600), ("u2".into(), "Bob".into(), 400)],
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.redistribute_cashbox(Request::new(proto::GetCashboxRequest { guild_id: "g".into() }))
        .await.unwrap().into_inner();
    assert!(resp.executed);
    assert_eq!(resp.redistribution_id, Some(rid.to_string()));
    assert_eq!(resp.total_amount, 1000);
    assert_eq!(resp.winners.len(), 2);
    assert_eq!(resp.winners[0].amount_won, 600);
}

// ── redistribute_due_cashboxes ──

#[tokio::test]
async fn redistribute_due_maps_and_clamps_negative() {
    let (s, c, cb, t, h) = defaults();
    cb.due_return.lock().unwrap().push((
        "g1".into(),
        RedistributionOutcome { redistribution_id: Uuid::new_v4(), total_amount: 100, winners: vec![] },
    ));
    let g = mk(s, c, cb, t, h);
    let resp = g.redistribute_due_cashboxes(Request::new(proto::RedistributeDueRequest {
        min_days_since_last: -5,
    })).await.unwrap().into_inner();
    assert_eq!(resp.redistributed.len(), 1);
    assert!(resp.redistributed[0].executed);
}

// ── deposit_cashbox ──

#[tokio::test]
async fn deposit_cashbox_delegates_with_source() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s, c, cb.clone(), t, h);
    g.deposit_cashbox(Request::new(proto::DepositCashboxRequest {
        guild_id: "g".into(), amount: 100,
        source: proto::CashboxDepositSource::CashboxSourceShopPurchase as i32,
    })).await.unwrap();
    let calls = cb.deposit_calls.lock().unwrap();
    assert_eq!(calls[0].0, "g");
    assert_eq!(calls[0].1, 100);
    assert!(matches!(calls[0].2, CashboxSource::ShopPurchase));
}

#[tokio::test]
async fn deposit_cashbox_rejects_unspecified_source() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s, c, cb, t, h);
    let err = g.deposit_cashbox(Request::new(proto::DepositCashboxRequest {
        guild_id: "g".into(), amount: 100,
        source: proto::CashboxDepositSource::CashboxSourceUnspecified as i32,
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[test]
fn proto_source_mapping_covers_all_variants() {
    use proto::CashboxDepositSource as P;
    assert!(proto_source_to_domain(P::CashboxSourceUnspecified as i32).is_none());
    assert!(matches!(proto_source_to_domain(P::CashboxSourceShopPurchase as i32), Some(CashboxSource::ShopPurchase)));
    assert!(matches!(proto_source_to_domain(P::CashboxSourceInsurancePurchase as i32), Some(CashboxSource::InsurancePurchase)));
    assert!(matches!(proto_source_to_domain(P::CashboxSourceProtectionPurchase as i32), Some(CashboxSource::ProtectionPurchase)));
    assert!(matches!(proto_source_to_domain(P::CashboxSourceBoostPurchase as i32), Some(CashboxSource::BoostPurchase)));
    assert!(matches!(proto_source_to_domain(P::CashboxSourceClassChangeCost as i32), Some(CashboxSource::ClassChangeCost)));
    assert!(matches!(proto_source_to_domain(P::CashboxSourceResetStatsCost as i32), Some(CashboxSource::ResetStatsCost)));
    assert!(matches!(proto_source_to_domain(P::CashboxSourceDonationTax as i32), Some(CashboxSource::DonationTax)));
    assert!(matches!(proto_source_to_domain(P::CashboxSourceCowardicePenalty as i32), Some(CashboxSource::CowardicePenalty)));
    assert!(matches!(proto_source_to_domain(P::CashboxSourceBetCommission as i32), Some(CashboxSource::BetCommission)));
    assert!(proto_source_to_domain(99999).is_none());
}

// ── track_steal_victim ──

#[tokio::test]
async fn track_steal_victim_none() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s, c, cb, t, h);
    let resp = g.track_steal_victim(Request::new(proto::TrackStealVictimRequest {
        guild_id: "g".into(), victim_id: "v".into(),
    })).await.unwrap();
    assert!(resp.into_inner().event.is_none());
}

#[tokio::test]
async fn track_steal_victim_some() {
    let (s, c, cb, t, h) = defaults();
    *t.steal_victim_return.lock().unwrap() = Some(TauntEvent {
        channel_id: "c".into(), target_user_id: "v".into(),
        message: "m".into(), nickname_suffix: "🤡".into(),
        streak_kind: "steal", streak_value: 3,
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.track_steal_victim(Request::new(proto::TrackStealVictimRequest {
        guild_id: "g".into(), victim_id: "v".into(),
    })).await.unwrap();
    let ev = resp.into_inner().event.unwrap();
    assert_eq!(ev.target_user_id, "v");
    assert_eq!(ev.streak_value, 3);
}

// ── track_steal_defended ──

#[tokio::test]
async fn track_steal_defended_delegates() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s, c, cb, t.clone(), h);
    g.track_steal_defended(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap();
    assert_eq!(t.defended_calls.lock().unwrap()[0], ("g".into(), "u".into()));
}

// ── get_taunts_config / set_* ──

#[tokio::test]
async fn get_taunts_config_maps() {
    let (s, c, cb, t, h) = defaults();
    *t.config.lock().unwrap() = Some(TauntsConfig {
        guild_id: "g".into(), channel_id: Some("c".into()),
        enabled: true, rename_enabled: true, messages_enabled: true,
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.get_taunts_config(Request::new(proto::GetTauntsConfigRequest {
        guild_id: "g".into(),
    })).await.unwrap().into_inner();
    assert_eq!(resp.guild_id, "g");
    assert_eq!(resp.channel_id.as_deref(), Some("c"));
    assert!(resp.enabled);
}

#[tokio::test]
async fn set_taunts_channel_passes_option() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s, c, cb, t.clone(), h);
    g.set_taunts_channel(Request::new(proto::SetTauntsChannelRequest {
        guild_id: "g".into(), channel_id: Some("c1".into()),
    })).await.unwrap();
    g.set_taunts_channel(Request::new(proto::SetTauntsChannelRequest {
        guild_id: "g".into(), channel_id: None,
    })).await.unwrap();
    let calls = t.set_channel_calls.lock().unwrap();
    assert_eq!(calls[0], ("g".into(), Some("c1".into())));
    assert_eq!(calls[1], ("g".into(), None));
}

#[tokio::test]
async fn set_taunts_enabled_delegates() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s, c, cb, t.clone(), h);
    g.set_taunts_enabled(Request::new(proto::SetTauntsEnabledRequest {
        guild_id: "g".into(), enabled: false,
    })).await.unwrap();
    assert_eq!(t.set_enabled_calls.lock().unwrap()[0], ("g".into(), false));
}

#[tokio::test]
async fn set_taunts_opt_out_delegates() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s, c, cb, t.clone(), h);
    g.set_taunts_opt_out(Request::new(proto::SetTauntsOptOutRequest {
        guild_id: "g".into(), user_id: "u".into(), opted_out: true,
    })).await.unwrap();
    assert_eq!(t.set_opt_out_calls.lock().unwrap()[0], ("g".into(), "u".into(), true));
}

// ── heist ──

#[tokio::test]
async fn attempt_heist_maps_success() {
    let (s, c, cb, t, h) = defaults();
    *h.outcome.lock().unwrap() = Some(HeistOutcome {
        success: true, chance_percent: 15, cashbox_total_before: 1000,
        amount_stolen: 300, tools_consumed: vec!["lockpick".into()],
        prison_released_at: None,
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.attempt_heist(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap().into_inner();
    assert!(resp.success);
    assert_eq!(resp.amount_stolen, 300);
    assert_eq!(resp.tools_consumed, vec!["lockpick".to_string()]);
    assert!(resp.prison_released_at.is_none());
}

#[tokio::test]
async fn attempt_heist_maps_failure_with_prison() {
    let (s, c, cb, t, h) = defaults();
    let release = Utc::now() + Duration::hours(24);
    *h.outcome.lock().unwrap() = Some(HeistOutcome {
        success: false, chance_percent: 5, cashbox_total_before: 1000,
        amount_stolen: 0, tools_consumed: vec![], prison_released_at: Some(release),
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.attempt_heist(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap().into_inner();
    assert!(!resp.success);
    assert_eq!(resp.prison_released_at, Some(release.to_rfc3339()));
}

#[tokio::test]
async fn get_heist_cooldown_maps() {
    let (s, c, cb, t, h) = defaults();
    let next = Utc::now() + Duration::hours(1);
    *h.cooldown.lock().unwrap() = Some(HeistCooldownStatus {
        ready: false, next_attempt_at: Some(next), last_success: Some(true),
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.get_heist_cooldown(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap().into_inner();
    assert!(!resp.ready);
    assert_eq!(resp.next_attempt_at, Some(next.to_rfc3339()));
    assert_eq!(resp.last_success, Some(true));
}

#[tokio::test]
async fn get_prison_status_maps() {
    let (s, c, cb, t, h) = defaults();
    let rel = Utc::now() + Duration::hours(2);
    *h.prison.lock().unwrap() = Some(PrisonStatusInfo {
        in_prison: true, released_at: Some(rel), reason: Some("heist_fail".into()),
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.get_prison_status(Request::new(proto::UserInGuildRequest {
        guild_id: "g".into(), user_id: "u".into(),
    })).await.unwrap().into_inner();
    assert!(resp.in_prison);
    assert_eq!(resp.released_at, Some(rel.to_rfc3339()));
    assert_eq!(resp.reason.as_deref(), Some("heist_fail"));
}

// ── trigger_daily_chaos ──

#[tokio::test]
async fn trigger_daily_chaos_none_returns_not_triggered() {
    let (s, c, cb, t, h) = defaults();
    let g = mk(s, c, cb, t, h);
    let resp = g.trigger_daily_chaos(Request::new(proto::TriggerDailyChaosRequest {
        guild_id: "g".into(),
    })).await.unwrap().into_inner();
    assert!(!resp.triggered);
}

#[tokio::test]
async fn trigger_daily_chaos_some_returns_fields() {
    let (s, c, cb, t, h) = defaults();
    *s.chaos_return.lock().unwrap() = Some(DailyChaosOutcome {
        loser_id: "l".into(), loser_name: "L".into(),
        winner_id: "w".into(), winner_name: "W".into(),
        amount: 500, channel_id: "c".into(),
        taunt_events: vec![TauntEvent {
            channel_id: "c".into(), target_user_id: "l".into(),
            message: "m".into(), nickname_suffix: "".into(),
            streak_kind: "bankruptcy", streak_value: 1,
        }],
    });
    let g = mk(s, c, cb, t, h);
    let resp = g.trigger_daily_chaos(Request::new(proto::TriggerDailyChaosRequest {
        guild_id: "g".into(),
    })).await.unwrap().into_inner();
    assert!(resp.triggered);
    assert_eq!(resp.loser_id, "l");
    assert_eq!(resp.winner_name, "W");
    assert_eq!(resp.amount, 500);
    assert_eq!(resp.channel_id, "c");
    assert_eq!(resp.taunt_events.len(), 1);
}
