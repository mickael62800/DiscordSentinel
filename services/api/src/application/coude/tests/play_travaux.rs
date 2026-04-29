use std::sync::Arc;
use std::sync::Mutex;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use sqlx::Postgres;
use sqlx::Transaction;
use crate::application::coude::play_travaux_service::PlayTravauxService;
use crate::domain::entities::coude::player::CombatStat;
use crate::domain::entities::coude::social::CoudeCurrentSeason;
use crate::domain::entities::coude::social::Event;
use crate::domain::entities::coude::heist::HeistAttempt;
use crate::domain::entities::coude::social::LeaderboardEntry;
use crate::domain::entities::coude::player::Player;
use crate::domain::entities::coude::heist::PrisonState;
use crate::domain::entities::coude::social::LeaderboardCategory;
use crate::domain::entities::coude::social::NewDailyChaos;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::entities::coude::travaux::TRAVAUX_COOLDOWN_KEY;
use crate::domain::entities::coude::travaux::TRAVAUX_COOLDOWN_SECS;
use crate::domain::enums::coude::coude_class::PlayerClass;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::casino::manage_wallet::TxWalletMutation;
use crate::ports::inbound::casino::manage_wallet::WalletMutation;
use crate::ports::inbound::coude::play_travaux::PlayTravauxCommand;
use crate::ports::inbound::coude::play_travaux::PlayTravauxUseCase;
use crate::ports::outbound::coude::heist_repository::HeistRepository;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::coude::social_repository::SocialRepository;
// ── Mocks ───────────────────────────────────────────────────────────

struct MockHeistRepo {
    prison: Mutex<Option<PrisonState>>,
}

#[async_trait]
impl HeistRepository for MockHeistRepo {
    async fn last_attempt(&self, _: &str, _: &str) -> Result<Option<HeistAttempt>, DomainError> { Ok(None) }
    async fn record_attempt(&self, _: &str, _: &str, _: bool, _: i64, _: i32, _: &[String]) -> Result<HeistAttempt, DomainError> { unimplemented!() }
    async fn get_prison(&self, _: &str, _: &str) -> Result<Option<PrisonState>, DomainError> {
        Ok(self.prison.lock().unwrap().clone())
    }
    async fn send_to_prison(&self, _: &str, _: &str, _: DateTime<Utc>, _: &str) -> Result<(), DomainError> { Ok(()) }
}

struct MockPlayerRepo {
    xp_calls: Mutex<Vec<(String, String, i64)>>,
}

#[async_trait]
impl PlayerRepository for MockPlayerRepo {
    async fn get_or_create(&self, g: &str, u: &str, name: &str) -> Result<Player, DomainError> {
        let now = Utc::now();
        Ok(Player {
            guild_id: g.into(), user_id: u.into(), username: name.into(),
            coins: 0, total_wins: 0, total_losses: 0, total_draws: 0,
            total_earned: 0, total_lost: 0, total_stolen: 0,
            cowardice_count: 0, chaos_events: 0, casino_wins: 0, casino_losses: 0,
            level: 1, xp: 0, stat_points: 0, atk: 0, def: 0,
            class: Some(PlayerClass::Tank), title: None, class_changed_at: None,
            hp_current: 100, hp_max: 100, hp_last_regen: None, repos_last_used: None,
            season: 1, created_at: now, updated_at: now,
        })
    }
    async fn get(&self, _: &str, _: &str) -> Result<Option<Player>, DomainError> { Ok(None) }
    async fn list(&self, _: &str, _: i64) -> Result<Vec<Player>, DomainError> { Ok(vec![]) }
    async fn random_active(&self, _: &str, _: i64, _: i64) -> Result<Vec<Player>, DomainError> { Ok(vec![]) }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
    async fn update_class(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn add_xp(&self, g: &str, u: &str, a: i64) -> Result<Option<XpProgress>, DomainError> {
        self.xp_calls.lock().unwrap().push((g.into(), u.into(), a));
        Ok(None)
    }
    async fn spend_stat_point(&self, _: &str, _: &str, _: CombatStat) -> Result<Option<Player>, DomainError> { Ok(None) }
    async fn reset_stats(&self, _: &str, _: &str, _: i64) -> Result<Option<Player>, DomainError> { Ok(None) }
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

struct MockSocialRepo {
    cooldown: Mutex<Option<DateTime<Utc>>>,
    set_calls: Mutex<Vec<(String, String, String, i64)>>,
}

#[async_trait]
impl SocialRepository for MockSocialRepo {
    async fn get_cooldown(&self, _: &str, _: &str, _: &str) -> Result<Option<DateTime<Utc>>, DomainError> {
        Ok(*self.cooldown.lock().unwrap())
    }
    async fn set_cooldown(&self, g: &str, u: &str, a: &str, d: i64) -> Result<(), DomainError> {
        self.set_calls.lock().unwrap().push((g.into(), u.into(), a.into(), d));
        Ok(())
    }
    async fn leaderboard(&self, _: &str, _: LeaderboardCategory, _: i64) -> Result<Vec<LeaderboardEntry>, DomainError> { Ok(vec![]) }
    async fn list_active_events(&self, _: &str) -> Result<Vec<Event>, DomainError> { Ok(vec![]) }
    async fn log_daily_chaos(&self, _: NewDailyChaos) -> Result<(), DomainError> { Ok(()) }
    async fn count_daily_chaos_today(&self, _: &str) -> Result<i64, DomainError> { Ok(0) }
    async fn get_or_bootstrap_current_season(&self, _: &str) -> Result<CoudeCurrentSeason, DomainError> {
        Ok(CoudeCurrentSeason { season_number: 1, started_at: Utc::now(), ends_at: Utc::now(), days_remaining: 30 })
    }
}

struct MockWalletUc {
    credit_calls: Mutex<Vec<(String, String, i64, String)>>,
}

#[async_trait]
impl ManageWalletUseCase for MockWalletUc {
    async fn credit(&self, g: &str, u: &str, a: i64, src: &str, _: &str) -> Result<WalletMutation, DomainError> {
        self.credit_calls.lock().unwrap().push((g.into(), u.into(), a, src.into()));
        Ok(WalletMutation { new_balance: a, previous_balance: 0, triggered_taunts: vec![] })
    }
    async fn debit(&self, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<WalletMutation, DomainError> { unimplemented!() }
    async fn transfer(&self, _: &str, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<Vec<TauntEvent>, DomainError> { Ok(vec![]) }
    async fn get_balance(&self, _: &str, _: &str) -> Result<i64, DomainError> { Ok(0) }
    async fn credit_tx(&self, _: &mut Transaction<'_, Postgres>, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<TxWalletMutation, DomainError> { unimplemented!() }
    async fn debit_tx(&self, _: &mut Transaction<'_, Postgres>, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<TxWalletMutation, DomainError> { unimplemented!() }
    async fn post_commit_taunts(&self, _: &str, _: &str, _: &TxWalletMutation) -> Vec<TauntEvent> { vec![] }
}

struct Harness {
    svc: PlayTravauxService,
    heist: Arc<MockHeistRepo>,
    player: Arc<MockPlayerRepo>,
    wallet: Arc<MockWalletUc>,
    social: Arc<MockSocialRepo>,
}

fn build(in_prison: bool, cooldown_active: bool) -> Harness {
    let prison = if in_prison {
        Some(PrisonState {
            guild_id: "g1".into(),
            user_id: "u1".into(),
            released_at: Utc::now() + Duration::hours(2),
            reason: "test".into(),
            created_at: Utc::now(),
        })
    } else {
        None
    };
    let heist = Arc::new(MockHeistRepo { prison: Mutex::new(prison) });
    let player = Arc::new(MockPlayerRepo { xp_calls: Mutex::new(vec![]) });
    let wallet = Arc::new(MockWalletUc { credit_calls: Mutex::new(vec![]) });
    let cd = if cooldown_active {
        Some(Utc::now() + Duration::hours(1))
    } else {
        None
    };
    let social = Arc::new(MockSocialRepo {
        cooldown: Mutex::new(cd),
        set_calls: Mutex::new(vec![]),
    });
    let svc = PlayTravauxService::new(
        heist.clone(),
        player.clone(),
        wallet.clone(),
        social.clone(),
    );
    Harness { svc, heist, player, wallet, social }
}

fn cmd() -> PlayTravauxCommand {
    PlayTravauxCommand {
        guild_id: "g1".into(),
        user_id: "u1".into(),
        username: "alice".into(),
    }
}

#[tokio::test]
async fn play_rejects_when_not_in_prison() {
    let h = build(false, false);
    let err = h.svc.play(cmd()).await.unwrap_err();
    assert!(matches!(err, DomainError::Forbidden(_)));
    assert!(h.wallet.credit_calls.lock().unwrap().is_empty());
    assert!(h.social.set_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn play_rejects_when_prison_expired() {
    // Released_at dans le passe -> is_active = false.
    let prison = PrisonState {
        guild_id: "g1".into(),
        user_id: "u1".into(),
        released_at: Utc::now() - Duration::hours(1),
        reason: "old".into(),
        created_at: Utc::now() - Duration::hours(3),
    };
    let h = build(false, false);
    *h.heist.prison.lock().unwrap() = Some(prison);
    let err = h.svc.play(cmd()).await.unwrap_err();
    assert!(matches!(err, DomainError::Forbidden(_)));
}

#[tokio::test]
async fn play_rejects_when_cooldown_active() {
    let h = build(true, true);
    let err = h.svc.play(cmd()).await.unwrap_err();
    assert!(matches!(err, DomainError::RateLimited(_)));
    assert!(h.wallet.credit_calls.lock().unwrap().is_empty());
    assert!(h.social.set_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn play_in_prison_returns_resolution_and_sets_cooldown() {
    let h = build(true, false);
    let res = h.svc.play(cmd()).await.unwrap();

    // Cooldown TOUJOURS pose, succes ou echec.
    let calls = h.social.set_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].2, TRAVAUX_COOLDOWN_KEY);
    assert_eq!(calls[0].3, TRAVAUX_COOLDOWN_SECS);

    // Task selectionnee dans le catalogue.
    assert!(["clean", "cook", "inform"].contains(&res.task_key));
    assert!(!res.task_label.is_empty());
    assert!(!res.flavor.is_empty());

    if res.success {
        assert!(res.coins_gain >= 50 && res.coins_gain <= 100);
        assert_eq!(res.xp_gain, 5);
        assert_eq!(h.wallet.credit_calls.lock().unwrap().len(), 1);
        assert_eq!(h.player.xp_calls.lock().unwrap().len(), 1);
    } else {
        assert_eq!(res.coins_gain, 0);
        assert_eq!(res.xp_gain, 0);
        assert_eq!(h.wallet.credit_calls.lock().unwrap().len(), 0);
        assert_eq!(h.player.xp_calls.lock().unwrap().len(), 0);
    }
}
