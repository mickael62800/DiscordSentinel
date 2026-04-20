use super::*;
use crate::domain::entities::{CombatStat, CoudePlayer, XpProgress};
use crate::ports::inbound::manage_coude_players::ManageCoudePlayersUseCase;
use crate::ports::outbound::CoudePlayerRepository;
use chrono::Utc;
use std::sync::Mutex as StdMutex;

#[derive(Default)]
struct MockRepo {
    random_count: StdMutex<Option<i64>>,
    random_min_coins: StdMutex<Option<i64>>,
    list_limit: StdMutex<Option<i64>>,
    update_class_returns: StdMutex<bool>,
    reset_stats_returns_some: StdMutex<bool>,
    player: StdMutex<Option<CoudePlayer>>,
}

impl MockRepo {
    fn with_update_class(ok: bool) -> Self {
        let m = Self::default();
        *m.update_class_returns.lock().unwrap() = ok;
        m
    }
}

fn sample_player() -> CoudePlayer {
    let now = Utc::now();
    CoudePlayer {
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "alice".into(),
        coins: 500,
        total_wins: 0,
        total_losses: 0,
        total_draws: 0,
        total_earned: 0,
        total_lost: 0,
        total_stolen: 0,
        cowardice_count: 0,
        chaos_events: 0,
        casino_wins: 0,
        casino_losses: 0,
        level: 1,
        xp: 100,
        stat_points: 2,
        atk: 0,
        def: 0,
        class: None,
        title: None,
        class_changed_at: None,
        hp_current: 100,
        hp_max: 100,
        hp_last_regen: None,
        repos_last_used: None,
        season: 1,
        created_at: now,
        updated_at: now,
    }
}

#[async_trait]
impl CoudePlayerRepository for MockRepo {
    async fn get_or_create(&self, _: &str, _: &str, _: &str) -> Result<CoudePlayer, DomainError> {
        Ok(sample_player())
    }
    async fn get(&self, _: &str, _: &str) -> Result<Option<CoudePlayer>, DomainError> {
        Ok(self.player.lock().unwrap().clone())
    }
    async fn list(&self, _: &str, limit: i64) -> Result<Vec<CoudePlayer>, DomainError> {
        *self.list_limit.lock().unwrap() = Some(limit);
        Ok(vec![])
    }
    async fn random_active(&self, _: &str, count: i64, min_coins: i64) -> Result<Vec<CoudePlayer>, DomainError> {
        *self.random_count.lock().unwrap() = Some(count);
        *self.random_min_coins.lock().unwrap() = Some(min_coins);
        Ok(vec![])
    }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
    async fn update_class(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(*self.update_class_returns.lock().unwrap())
    }
    async fn add_xp(&self, _: &str, _: &str, _: i64) -> Result<Option<XpProgress>, DomainError> { Ok(None) }
    async fn spend_stat_point(&self, _: &str, _: &str, _: CombatStat) -> Result<Option<CoudePlayer>, DomainError> { Ok(None) }
    async fn reset_stats(&self, _: &str, _: &str, _: i64) -> Result<Option<CoudePlayer>, DomainError> {
        if *self.reset_stats_returns_some.lock().unwrap() {
            Ok(Some(sample_player()))
        } else {
            Ok(None)
        }
    }
    async fn record_coins_earned(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn record_coins_lost(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn record_win(&self, _: &str, _: &str, _: i64, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn record_loss(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn record_draw(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> { Ok(true) }
    async fn touch_win_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn touch_loss_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn reset_combat_streaks(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn touch_steal_victim_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn reset_steal_victim_streak(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn touch_bj_win_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn touch_bj_bust_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn reset_bj_bust_streak(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn increment_cowardice(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> { Ok(None) }
    async fn increment_chaos(&self, _: &str, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn update_hp(&self, _: &str, _: &str, _: i32, _: i32) -> Result<(), DomainError> { Ok(()) }
    async fn full_heal(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn regen_hp_tick(&self, _: f64, _: f64, _: f64, _: f64) -> Result<u64, DomainError> { Ok(0) }
}

// ── update_class ──

#[tokio::test]
async fn update_class_rejects_empty() {
    let svc = ManageCoudePlayersService::new(Arc::new(MockRepo::default()));
    assert!(matches!(svc.update_class("g", "u", "").await, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn update_class_rejects_whitespace_only() {
    let svc = ManageCoudePlayersService::new(Arc::new(MockRepo::default()));
    assert!(matches!(svc.update_class("g", "u", "   ").await, Err(DomainError::ValidationError(_))));
    assert!(matches!(svc.update_class("g", "u", "\t\n").await, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn update_class_accepts_valid_and_returns_ok() {
    let svc = ManageCoudePlayersService::new(Arc::new(MockRepo::with_update_class(true)));
    assert!(svc.update_class("g", "u", "bourrin").await.is_ok());
}

#[tokio::test]
async fn update_class_maps_not_found_when_repo_returns_false() {
    let svc = ManageCoudePlayersService::new(Arc::new(MockRepo::with_update_class(false)));
    assert!(matches!(svc.update_class("g", "u", "bourrin").await, Err(DomainError::NotFound(_))));
}

// ── random_active clamping ──

#[tokio::test]
async fn random_active_clamps_count_upper() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudePlayersService::new(repo.clone());
    svc.random_active("g", 100).await.unwrap();
    assert_eq!(*repo.random_count.lock().unwrap(), Some(50));
}

#[tokio::test]
async fn random_active_clamps_count_lower() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudePlayersService::new(repo.clone());
    svc.random_active("g", 0).await.unwrap();
    assert_eq!(*repo.random_count.lock().unwrap(), Some(1));
    svc.random_active("g", -5).await.unwrap();
    assert_eq!(*repo.random_count.lock().unwrap(), Some(1));
}

#[tokio::test]
async fn random_active_passes_min_coins_50() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudePlayersService::new(repo.clone());
    svc.random_active("g", 10).await.unwrap();
    assert_eq!(*repo.random_min_coins.lock().unwrap(), Some(50));
}

// ── list uses legacy limit 200 ──

#[tokio::test]
async fn list_uses_limit_200() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudePlayersService::new(repo.clone());
    svc.list("g").await.unwrap();
    assert_eq!(*repo.list_limit.lock().unwrap(), Some(200));
}

// ── reset_stats validation ──

#[tokio::test]
async fn reset_stats_rejects_negative_cost() {
    let svc = ManageCoudePlayersService::new(Arc::new(MockRepo::default()));
    assert!(matches!(svc.reset_stats("g", "u", -1).await, Err(DomainError::ValidationError(_))));
    assert!(matches!(svc.reset_stats("g", "u", -999).await, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn reset_stats_accepts_zero_cost() {
    let repo = MockRepo::default();
    *repo.reset_stats_returns_some.lock().unwrap() = true;
    let svc = ManageCoudePlayersService::new(Arc::new(repo));
    assert!(svc.reset_stats("g", "u", 0).await.is_ok());
}

// ── spend_stat_point maps None to ValidationError ──

#[tokio::test]
async fn spend_stat_point_none_returns_validation_error() {
    let svc = ManageCoudePlayersService::new(Arc::new(MockRepo::default()));
    let err = svc.spend_stat_point("g", "u", CombatStat::Atk).await.unwrap_err();
    match err {
        DomainError::ValidationError(msg) => {
            assert!(msg.contains("introuvable") || msg.contains("stat_points"));
        }
        other => panic!("Expected ValidationError, got {:?}", other),
    }
}

// ── add_xp maps None to NotFound ──

#[tokio::test]
async fn add_xp_none_returns_not_found() {
    let svc = ManageCoudePlayersService::new(Arc::new(MockRepo::default()));
    assert!(matches!(svc.add_xp("g", "u", 100).await, Err(DomainError::NotFound(_))));
}

// ── get player not found ──

#[tokio::test]
async fn get_not_found_returns_not_found_error() {
    let svc = ManageCoudePlayersService::new(Arc::new(MockRepo::default()));
    assert!(matches!(svc.get("g", "u").await, Err(DomainError::NotFound(_))));
}
