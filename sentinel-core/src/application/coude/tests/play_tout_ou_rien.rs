use crate::application::coude::play_tout_ou_rien_service::PlayToutOuRienService;
use crate::domain::entities::coude::player::CombatStat;
use crate::domain::entities::coude::player::Player;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::entities::coude::social::Event;
use crate::domain::entities::coude::social::LeaderboardCategory;
use crate::domain::entities::coude::social::LeaderboardEntry;
use crate::domain::entities::coude::social::NewDailyChaos;
use crate::domain::entities::coude::social::Season;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::entities::coude::tout_ou_rien::ToutOuRienOutcome;
use crate::domain::entities::coude::tout_ou_rien_log::ToutOuRienLogEntry;
use crate::domain::entities::coude::tout_ou_rien_log::ToutOuRienLogOutcome;
use crate::domain::entities::coude::tout_ou_rien_log::ToutOuRienUserStats;
use crate::domain::enums::coude::coude_class::PlayerClass;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::casino::manage_wallet::TxWalletMutation;
use crate::ports::inbound::casino::manage_wallet::WalletMutation;
use crate::ports::inbound::coude::play_tout_ou_rien::PlayToutOuRienCommand;
use crate::ports::inbound::coude::play_tout_ou_rien::PlayToutOuRienUseCase;
use crate::ports::inbound::coude::play_tout_ou_rien::MIN_BALANCE_FOR_PLAY;
use crate::ports::outbound::coude::player_repository::PlayerRepository;
use crate::ports::outbound::coude::social_repository::SocialRepository;
use crate::ports::outbound::coude::tout_ou_rien_repository::ToutOuRienRepository;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use std::sync::Arc;
use std::sync::Mutex;
// ── Mocks (minimal — seules les methodes utilisees sont implementees) ─

struct MockPlayerRepo {
    coins: Mutex<i64>,
}

#[async_trait]
impl PlayerRepository for MockPlayerRepo {
    async fn get_or_create(&self, g: &str, u: &str, name: &str) -> Result<Player, DomainError> {
        let now = Utc::now();
        Ok(Player {
            guild_id: g.into(),
            user_id: u.into(),
            username: name.into(),
            coins: *self.coins.lock().unwrap(),
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
            xp: 0,
            stat_points: 0,
            atk: 0,
            def: 0,
            class: Some(PlayerClass::Tank),
            title: None,
            class_changed_at: None,
            hp_current: 100,
            hp_max: 100,
            hp_last_regen: None,
            repos_last_used: None,
            season: 1,
            created_at: now,
            updated_at: now,
        })
    }
    async fn get(&self, _: &str, _: &str) -> Result<Option<Player>, DomainError> {
        Ok(None)
    }
    async fn list(&self, _: &str, _: i64) -> Result<Vec<Player>, DomainError> {
        Ok(vec![])
    }
    async fn random_active(&self, _: &str, _: i64, _: i64) -> Result<Vec<Player>, DomainError> {
        Ok(vec![])
    }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> {
        Ok(vec![])
    }
    async fn update_class(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn add_xp(&self, _: &str, _: &str, _: i64) -> Result<Option<XpProgress>, DomainError> {
        Ok(None)
    }
    async fn spend_stat_point(
        &self,
        _: &str,
        _: &str,
        _: CombatStat,
    ) -> Result<Option<Player>, DomainError> {
        Ok(None)
    }
    async fn reset_stats(&self, _: &str, _: &str, _: i64) -> Result<Option<Player>, DomainError> {
        Ok(None)
    }
    async fn record_coins_earned(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn record_coins_lost(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn record_win(&self, _: &str, _: &str, _: i64, _: i64) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn record_loss(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn record_draw(&self, _: &str, _: &str, _: i64) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn touch_win_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }
    async fn touch_loss_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }
    async fn reset_combat_streaks(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn touch_steal_victim_streak(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }
    async fn reset_steal_victim_streak(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn touch_bj_win_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }
    async fn touch_bj_bust_streak(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }
    async fn reset_bj_bust_streak(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn increment_cowardice(&self, _: &str, _: &str) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }
    async fn increment_chaos(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn update_hp(&self, _: &str, _: &str, _: i32, _: i32) -> Result<(), DomainError> {
        Ok(())
    }
    async fn full_heal(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn regen_hp_tick(&self, _: f64, _: f64, _: f64, _: f64) -> Result<u64, DomainError> {
        Ok(0)
    }
}

struct MockSocialRepo {
    cooldown: Mutex<Option<DateTime<Utc>>>,
    // Enregistre les claims REUSSIS (cooldown effectivement pose).
    set_calls: Mutex<Vec<(String, String, String, i64)>>,
    // Enregistre les liberations de claim (clear_cooldown).
    clear_calls: Mutex<Vec<(String, String, String)>>,
}

#[async_trait]
impl SocialRepository for MockSocialRepo {
    async fn get_cooldown(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        Ok(*self.cooldown.lock().unwrap())
    }
    async fn set_cooldown(&self, g: &str, u: &str, a: &str, d: i64) -> Result<(), DomainError> {
        self.set_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), a.into(), d));
        Ok(())
    }
    async fn try_claim_cooldown(
        &self,
        g: &str,
        u: &str,
        key: &str,
        ttl: i64,
    ) -> Result<bool, DomainError> {
        // Claim perdu si un cooldown actif existe deja.
        if self.cooldown.lock().unwrap().is_some() {
            return Ok(false);
        }
        // Claim gagne : on enregistre la pose effective du cooldown.
        self.set_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), key.into(), ttl));
        Ok(true)
    }
    async fn clear_cooldown(&self, g: &str, u: &str, key: &str) -> Result<(), DomainError> {
        self.clear_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), key.into()));
        Ok(())
    }
    async fn leaderboard(
        &self,
        _: &str,
        _: LeaderboardCategory,
        _: i64,
    ) -> Result<Vec<LeaderboardEntry>, DomainError> {
        Ok(vec![])
    }
    async fn list_active_events(&self, _: &str) -> Result<Vec<Event>, DomainError> {
        Ok(vec![])
    }
    async fn log_daily_chaos(&self, _: NewDailyChaos) -> Result<(), DomainError> {
        Ok(())
    }
    async fn count_daily_chaos_today(&self, _: &str) -> Result<i64, DomainError> {
        Ok(0)
    }
    async fn get_or_bootstrap_current_season(&self, _: &str) -> Result<Season, DomainError> {
        Ok(Season {
            season_number: 1,
            started_at: Utc::now(),
            ends_at: Utc::now(),
            days_remaining: 30,
        })
    }
}

struct MockWalletUc {
    credit_calls: Mutex<Vec<(String, String, i64, String)>>,
    debit_calls: Mutex<Vec<(String, String, i64, String)>>,
}

#[async_trait]
impl ManageWalletUseCase for MockWalletUc {
    async fn credit(
        &self,
        g: &str,
        u: &str,
        a: i64,
        src: &str,
        _: &str,
    ) -> Result<WalletMutation, DomainError> {
        self.credit_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), a, src.into()));
        Ok(WalletMutation {
            new_balance: a,
            previous_balance: 0,
            triggered_taunts: vec![],
        })
    }
    async fn debit(
        &self,
        g: &str,
        u: &str,
        a: i64,
        src: &str,
        _: &str,
    ) -> Result<WalletMutation, DomainError> {
        self.debit_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), a, src.into()));
        Ok(WalletMutation {
            new_balance: 0,
            previous_balance: a,
            triggered_taunts: vec![],
        })
    }
    async fn transfer(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<Vec<TauntEvent>, DomainError> {
        Ok(vec![])
    }
    async fn get_balance(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(0)
    }
    async fn credit_tx(
        &self,
        _: &mut dyn crate::ports::uow::DbTx,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<TxWalletMutation, DomainError> {
        unimplemented!()
    }
    async fn debit_tx(
        &self,
        _: &mut dyn crate::ports::uow::DbTx,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<TxWalletMutation, DomainError> {
        unimplemented!()
    }
    async fn post_commit_taunts(&self, _: &str, _: &str, _: &TxWalletMutation) -> Vec<TauntEvent> {
        vec![]
    }
}

struct MockLogRepo {
    records: Mutex<Vec<(String, String, i64, ToutOuRienLogOutcome, i64)>>,
}

#[async_trait]
impl ToutOuRienRepository for MockLogRepo {
    async fn record(
        &self,
        g: &str,
        u: &str,
        _name: &str,
        mise: i64,
        outcome: ToutOuRienLogOutcome,
        delta: i64,
    ) -> Result<(), DomainError> {
        self.records
            .lock()
            .unwrap()
            .push((g.into(), u.into(), mise, outcome, delta));
        Ok(())
    }
    async fn memorial(&self, _: &str, _: i64) -> Result<Vec<ToutOuRienLogEntry>, DomainError> {
        Ok(vec![])
    }
    async fn user_stats(&self, _: &str, _: &str) -> Result<ToutOuRienUserStats, DomainError> {
        Ok(ToutOuRienUserStats {
            attempts: 0,
            wins: 0,
            losses: 0,
            biggest_win: 0,
            biggest_loss: 0,
        })
    }
}

struct Harness {
    svc: PlayToutOuRienService,
    wallet: Arc<MockWalletUc>,
    social: Arc<MockSocialRepo>,
    log: Arc<MockLogRepo>,
}

fn build(coins: i64, cooldown: Option<DateTime<Utc>>) -> Harness {
    let player = Arc::new(MockPlayerRepo {
        coins: Mutex::new(coins),
    });
    let wallet = Arc::new(MockWalletUc {
        credit_calls: Mutex::new(vec![]),
        debit_calls: Mutex::new(vec![]),
    });
    let social = Arc::new(MockSocialRepo {
        cooldown: Mutex::new(cooldown),
        set_calls: Mutex::new(vec![]),
        clear_calls: Mutex::new(vec![]),
    });
    let log = Arc::new(MockLogRepo {
        records: Mutex::new(vec![]),
    });
    let svc =
        PlayToutOuRienService::new(player.clone(), wallet.clone(), social.clone(), log.clone());
    Harness {
        svc,
        wallet,
        social,
        log,
    }
}

fn cmd() -> PlayToutOuRienCommand {
    PlayToutOuRienCommand {
        guild_id: "g1".into(),
        user_id: "u1".into(),
        username: "alice".into(),
    }
}

#[tokio::test]
async fn play_rejects_when_cooldown_active() {
    let h = build(1000, Some(Utc::now() + chrono::Duration::days(3)));
    let err = h.svc.play(cmd()).await.unwrap_err();
    assert!(matches!(err, DomainError::RateLimited(_)));
    // Pas de mutation wallet ni de log.
    assert!(h.wallet.credit_calls.lock().unwrap().is_empty());
    assert!(h.wallet.debit_calls.lock().unwrap().is_empty());
    assert!(h.log.records.lock().unwrap().is_empty());
    assert!(h.social.set_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn play_rejects_when_balance_below_min() {
    let h = build(MIN_BALANCE_FOR_PLAY - 1, None);
    let err = h.svc.play(cmd()).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
    assert!(h.wallet.credit_calls.lock().unwrap().is_empty());
    assert!(h.wallet.debit_calls.lock().unwrap().is_empty());
    assert!(h.social.set_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn play_with_min_balance_passes_validation_and_emits_one_mutation() {
    // 50/50 random : on ne peut pas predire Win/Lose, mais on peut verifier
    // qu'il y a EXACTEMENT une mutation wallet (credit OU debit), pas zero.
    let h = build(MIN_BALANCE_FOR_PLAY, None);
    let res = h.svc.play(cmd()).await.unwrap();

    let credit_count = h.wallet.credit_calls.lock().unwrap().len();
    let debit_count = h.wallet.debit_calls.lock().unwrap().len();
    assert_eq!(
        credit_count + debit_count,
        1,
        "exactement une mutation wallet"
    );

    // Cooldown pose.
    let calls = h.social.set_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].2, "tout_ou_rien");
    assert_eq!(calls[0].3, 7 * 24 * 3600);

    // Memorial logge.
    assert_eq!(h.log.records.lock().unwrap().len(), 1);

    // initial_coins reportes correctement.
    assert_eq!(res.initial_coins, MIN_BALANCE_FOR_PLAY);
    // final_balance jamais negatif.
    assert!(res.final_balance >= 0);

    match res.outcome {
        ToutOuRienOutcome::Win => {
            assert_eq!(res.delta, MIN_BALANCE_FOR_PLAY);
            assert_eq!(credit_count, 1);
            assert_eq!(debit_count, 0);
        }
        ToutOuRienOutcome::Lose => {
            // -80% du wallet.
            assert_eq!(res.delta, -((MIN_BALANCE_FOR_PLAY as f64 * 0.8) as i64));
            assert_eq!(credit_count, 0);
            assert_eq!(debit_count, 1);
        }
    }
}

#[tokio::test]
async fn play_records_memorial_with_initial_balance() {
    let h = build(500, None);
    let res = h.svc.play(cmd()).await.unwrap();
    let records = h.log.records.lock().unwrap();
    assert_eq!(records.len(), 1);
    let (g, u, mise, outcome, delta) = &records[0];
    assert_eq!(g, "g1");
    assert_eq!(u, "u1");
    assert_eq!(*mise, 500);
    assert_eq!(*delta, res.delta);
    match res.outcome {
        ToutOuRienOutcome::Win => assert_eq!(*outcome, ToutOuRienLogOutcome::Won),
        ToutOuRienOutcome::Lose => assert_eq!(*outcome, ToutOuRienLogOutcome::Lost),
    }
}

#[tokio::test]
async fn cooldown_set_only_after_successful_play() {
    // Si la balance est trop basse, on ne pose pas le cooldown.
    let h = build(50, None);
    let _ = h.svc.play(cmd()).await;
    assert!(h.social.set_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn player_repo_called_with_command_args() {
    // Simple sanity : on s'assure que le service appelle bien
    // get_or_create avec les args de la commande (la balance lue est
    // celle de coins=1000).
    let h = build(1000, None);
    let _ = h.svc.play(cmd()).await.unwrap();
    // get_or_create est cable mais n'a pas de tracker dans MockPlayerRepo —
    // le check indirect : la mutation wallet utilise les bons IDs.
    let calls: Vec<_> = h
        .wallet
        .credit_calls
        .lock()
        .unwrap()
        .iter()
        .chain(h.wallet.debit_calls.lock().unwrap().iter())
        .cloned()
        .collect();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "g1");
    assert_eq!(calls[0].1, "u1");
}
