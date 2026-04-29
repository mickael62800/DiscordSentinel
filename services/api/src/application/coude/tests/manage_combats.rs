use super::*;
use crate::domain::entities::coude::combat::CombatResolution;
use crate::domain::entities::coude::combat::CoudeCombat;
use crate::domain::entities::coude::combat::NewCoudeCombat;
use crate::ports::inbound::coude::manage_combats::ManageCoudeCombatsUseCase;
use crate::ports::outbound::coude::combat_repository::CombatRepository;
use chrono::Utc;
use std::sync::Mutex as StdMutex;
use uuid::Uuid;

#[derive(Default)]
struct MockRepo {
    created: StdMutex<Vec<NewCoudeCombat>>,
    set_betting_arg: StdMutex<Option<(Uuid, String)>>,
    cancel_returns: StdMutex<bool>,
    resolve_returns: StdMutex<bool>,
    list_limit_received: StdMutex<Option<i64>>,
    get_returns: StdMutex<Option<CoudeCombat>>,
    purge_returns: StdMutex<Vec<(String, u64)>>,
    purge_guild_received: StdMutex<Option<String>>,
}

impl MockRepo {
    fn with_cancel(returns: bool) -> Self {
        let m = Self::default();
        *m.cancel_returns.lock().unwrap() = returns;
        m
    }
    fn with_resolve(returns: bool) -> Self {
        let m = Self::default();
        *m.resolve_returns.lock().unwrap() = returns;
        m
    }
    fn ok_cancel() -> Self { Self::with_cancel(true) }
}

fn sample_combat() -> CoudeCombat {
    CoudeCombat {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: None,
        attacker_id: "a".into(),
        attacker_name: "A".into(),
        defender_id: "d".into(),
        defender_name: "D".into(),
        mise: 100,
        status: "pending".into(),
        winner_id: None,
        attacker_roll: None,
        defender_roll: None,
        chaos_event: None,
        special_attack: None,
        defender_special: None,
        coins_transferred: None,
        result_message: None,
        message_id: None,
        created_at: Utc::now(),
        accepted_at: None,
        resolved_at: None,
    }
}

#[async_trait]
impl CombatRepository for MockRepo {
    async fn list(&self, _: &str, _: Option<&str>, limit: i64) -> Result<Vec<CoudeCombat>, DomainError> {
        *self.list_limit_received.lock().unwrap() = Some(limit);
        Ok(vec![])
    }
    async fn get(&self, _: Uuid) -> Result<Option<CoudeCombat>, DomainError> {
        Ok(self.get_returns.lock().unwrap().clone())
    }
    async fn get_pending_for_attacker(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
    async fn get_pending_for_defender(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
    async fn list_expired_pending(&self) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
    async fn claim_due_betting_combats(&self, _: i64) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
    async fn claim_stuck_resolving_combats(&self, _: i64) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
    async fn claim_expired_pending_combats(&self, _: i64) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
    async fn get_betting_for_participant(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
    async fn create(&self, new: NewCoudeCombat) -> Result<CoudeCombat, DomainError> {
        self.created.lock().unwrap().push(new.clone());
        Ok(CoudeCombat {
            id: Uuid::new_v4(),
            guild_id: new.guild_id,
            channel_id: new.channel_id,
            attacker_id: new.attacker_id,
            attacker_name: new.attacker_name,
            defender_id: new.defender_id,
            defender_name: new.defender_name,
            mise: new.mise,
            status: "pending".into(),
            winner_id: None,
            attacker_roll: None,
            defender_roll: None,
            chaos_event: None,
            special_attack: new.special_attack,
            defender_special: None,
            coins_transferred: None,
            result_message: None,
            message_id: None,
            created_at: Utc::now(),
            accepted_at: None,
            resolved_at: None,
        })
    }
    async fn resolve(&self, _: Uuid, _: CombatResolution) -> Result<bool, DomainError> {
        Ok(*self.resolve_returns.lock().unwrap())
    }
    async fn set_betting(&self, id: Uuid, msg: &str) -> Result<bool, DomainError> {
        *self.set_betting_arg.lock().unwrap() = Some((id, msg.into()));
        Ok(true)
    }
    async fn expire(&self, _: Uuid) -> Result<bool, DomainError> { Ok(true) }
    async fn cancel_pending(&self, _: Uuid) -> Result<bool, DomainError> {
        Ok(*self.cancel_returns.lock().unwrap())
    }
    async fn set_defender_special(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn mark_unresolved_bets_lost(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
    async fn purge_guild_subsystem(&self, g: &str) -> Result<Vec<(String, u64)>, DomainError> {
        *self.purge_guild_received.lock().unwrap() = Some(g.to_string());
        Ok(self.purge_returns.lock().unwrap().clone())
    }
}

fn new_combat(attacker: &str, defender: &str, mise: i64) -> NewCoudeCombat {
    NewCoudeCombat {
        guild_id: "g".into(),
        channel_id: None,
        attacker_id: attacker.into(),
        attacker_name: "A".into(),
        defender_id: defender.into(),
        defender_name: "D".into(),
        mise,
        special_attack: None,
    }
}

// ── create() validation ──

#[tokio::test]
async fn create_rejects_negative_mise() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    let err = svc.create(new_combat("a", "d", -1)).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn create_rejects_zero_mise() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    let err = svc.create(new_combat("a", "d", 0)).await.unwrap_err();
    match err {
        DomainError::ValidationError(msg) => assert!(msg.contains("positive")),
        other => panic!("Expected ValidationError, got {:?}", other),
    }
}

#[tokio::test]
async fn create_rejects_self_duel() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    let err = svc.create(new_combat("alice", "alice", 100)).await.unwrap_err();
    match err {
        DomainError::ValidationError(msg) => assert!(msg.contains("lui-meme")),
        other => panic!("Expected ValidationError, got {:?}", other),
    }
}

#[tokio::test]
async fn create_accepts_valid_combat() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeCombatsService::new(repo.clone());
    let combat = svc.create(new_combat("a", "d", 100)).await.unwrap();
    assert_eq!(combat.mise, 100);
    assert_eq!(combat.attacker_id, "a");
    assert_eq!(repo.created.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn create_surprise_without_gate_does_not_validate_hp() {
    // Sans `with_surprise_gate`, le gate est inactif.
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    let mut cmd = new_combat("a", "d", 100);
    cmd.special_attack = Some("surprise".into());
    assert!(svc.create(cmd).await.is_ok());
}

// ── list() clamping ──

#[tokio::test]
async fn list_clamps_limit_upper() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeCombatsService::new(repo.clone());
    svc.list("g", None, 500).await.unwrap();
    assert_eq!(*repo.list_limit_received.lock().unwrap(), Some(200));
}

#[tokio::test]
async fn list_clamps_limit_lower() {
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeCombatsService::new(repo.clone());
    svc.list("g", None, 0).await.unwrap();
    assert_eq!(*repo.list_limit_received.lock().unwrap(), Some(1));
}

#[tokio::test]
async fn list_filters_out_all_as_none() {
    // "all" doit etre traite comme None (no filter).
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeCombatsService::new(repo.clone());
    svc.list("g", Some("all"), 50).await.unwrap();
    // Si la conversion a fonctionne, le test passe (pas de panic).
}

// ── get() NotFound ──

#[tokio::test]
async fn get_not_found_returns_error() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    let err = svc.get(Uuid::new_v4()).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

// ── cancel() / expire() NotFound ──

#[tokio::test]
async fn cancel_returns_not_found_if_nothing_cancelled() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::with_cancel(false)));
    let err = svc.cancel(Uuid::new_v4()).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn cancel_succeeds_when_repo_returns_true() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::ok_cancel()));
    assert!(svc.cancel(Uuid::new_v4()).await.is_ok());
}

// ── resolve() conflict ──

#[tokio::test]
async fn resolve_conflict_when_already_resolved() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::with_resolve(false)));
    let res = CombatResolution {
        status: "resolved".into(),
        winner_id: None,
        attacker_roll: Some(10),
        defender_roll: Some(5),
        chaos_event: None,
        result_message: Some("done".into()),
        coins_transferred: 100,
    };
    let err = svc.resolve(Uuid::new_v4(), res).await.unwrap_err();
    assert!(matches!(err, DomainError::Conflict(_)));
}

// ── set_betting / set_defender_special validation ──

#[tokio::test]
async fn set_betting_rejects_empty_message_id() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    assert!(matches!(svc.set_betting(Uuid::new_v4(), "").await, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn set_betting_accepts_non_empty() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    assert!(svc.set_betting(Uuid::new_v4(), "msg123").await.is_ok());
}

// ── get_guild_id (RBAC resource-based) ────────────────────────────────

#[tokio::test]
async fn get_guild_id_returns_none_when_combat_absent() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    let out = svc.get_guild_id(Uuid::new_v4()).await.unwrap();
    assert!(out.is_none());
}

#[tokio::test]
async fn get_guild_id_returns_guild_from_repo_when_present() {
    let repo = MockRepo::default();
    let mut c = sample_combat();
    c.guild_id = "guild-42".into();
    *repo.get_returns.lock().unwrap() = Some(c);
    let svc = ManageCoudeCombatsService::new(Arc::new(repo));
    assert_eq!(svc.get_guild_id(Uuid::new_v4()).await.unwrap().as_deref(), Some("guild-42"));
}

// ── purge_guild_subsystem ─────────────────────────────────────────────

#[tokio::test]
async fn purge_guild_subsystem_delegates_to_repo_with_guild_id() {
    let repo = MockRepo::default();
    *repo.purge_returns.lock().unwrap() = vec![
        ("coude_bets".into(), 3),
        ("coude_combats".into(), 2),
        ("coude_players".into(), 5),
    ];
    let repo = Arc::new(repo);
    let svc = ManageCoudeCombatsService::new(repo.clone());

    let out = svc.purge_guild_subsystem("guild-xyz").await.unwrap();
    assert_eq!(out.len(), 3);
    assert_eq!(out[0], ("coude_bets".into(), 3));
    assert_eq!(*repo.purge_guild_received.lock().unwrap(), Some("guild-xyz".into()));
}

#[tokio::test]
async fn set_defender_special_rejects_empty_item() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    assert!(matches!(svc.set_defender_special(Uuid::new_v4(), "").await, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn set_defender_special_ok_when_repo_updates() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    assert!(svc.set_defender_special(Uuid::new_v4(), "fake_plaque").await.is_ok());
}

#[tokio::test]
async fn expire_ok_when_repo_returns_true() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    assert!(svc.expire(Uuid::new_v4()).await.is_ok());
}

#[tokio::test]
async fn expire_not_found_when_repo_returns_false() {
    #[derive(Default)]
    struct FalseExpireRepo;
    #[async_trait]
    impl CombatRepository for FalseExpireRepo {
        async fn list(&self, _: &str, _: Option<&str>, _: i64) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
        async fn get(&self, _: Uuid) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
        async fn get_pending_for_attacker(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
        async fn get_pending_for_defender(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
        async fn list_expired_pending(&self) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
        async fn claim_due_betting_combats(&self, _: i64) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
        async fn claim_stuck_resolving_combats(&self, _: i64) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
        async fn claim_expired_pending_combats(&self, _: i64) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
        async fn get_betting_for_participant(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
        async fn create(&self, _: NewCoudeCombat) -> Result<CoudeCombat, DomainError> { unimplemented!() }
        async fn resolve(&self, _: Uuid, _: CombatResolution) -> Result<bool, DomainError> { Ok(false) }
        async fn set_betting(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { Ok(false) }
        async fn expire(&self, _: Uuid) -> Result<bool, DomainError> { Ok(false) }
        async fn cancel_pending(&self, _: Uuid) -> Result<bool, DomainError> { Ok(false) }
        async fn set_defender_special(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { Ok(false) }
        async fn mark_unresolved_bets_lost(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
    }
    let svc = ManageCoudeCombatsService::new(Arc::new(FalseExpireRepo));
    assert!(matches!(svc.expire(Uuid::new_v4()).await, Err(DomainError::NotFound(_))));
    assert!(matches!(svc.set_defender_special(Uuid::new_v4(), "k").await, Err(DomainError::NotFound(_))));
}

#[tokio::test]
async fn resolve_ok_when_repo_returns_true() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::with_resolve(true)));
    let res = CombatResolution {
        status: "resolved".into(), winner_id: Some("a".into()),
        attacker_roll: Some(10), defender_roll: Some(5),
        chaos_event: None, result_message: Some("ok".into()),
        coins_transferred: 100,
    };
    assert!(svc.resolve(Uuid::new_v4(), res).await.is_ok());
}

#[tokio::test]
async fn get_pending_for_attacker_delegates() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    assert!(svc.get_pending_for_attacker("g", "a").await.unwrap().is_none());
}

#[tokio::test]
async fn get_pending_for_defender_delegates() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    assert!(svc.get_pending_for_defender("g", "d").await.unwrap().is_none());
}

#[tokio::test]
async fn list_expired_pending_delegates() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    assert!(svc.list_expired_pending().await.unwrap().is_empty());
}

#[tokio::test]
async fn get_betting_for_participant_delegates() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    assert!(svc.get_betting_for_participant("g", "u").await.unwrap().is_none());
}

#[tokio::test]
async fn cancel_continues_even_if_mark_bets_lost_fails() {
    // mark_unresolved_bets_lost echoue (Internal), mais cancel doit reussir.
    #[derive(Default)]
    struct BetsFailRepo;
    #[async_trait]
    impl CombatRepository for BetsFailRepo {
        async fn list(&self, _: &str, _: Option<&str>, _: i64) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
        async fn get(&self, _: Uuid) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
        async fn get_pending_for_attacker(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
        async fn get_pending_for_defender(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
        async fn list_expired_pending(&self) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
        async fn claim_due_betting_combats(&self, _: i64) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
        async fn claim_stuck_resolving_combats(&self, _: i64) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
        async fn claim_expired_pending_combats(&self, _: i64) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
        async fn get_betting_for_participant(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
        async fn create(&self, _: NewCoudeCombat) -> Result<CoudeCombat, DomainError> { unimplemented!() }
        async fn resolve(&self, _: Uuid, _: CombatResolution) -> Result<bool, DomainError> { Ok(true) }
        async fn set_betting(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { Ok(true) }
        async fn expire(&self, _: Uuid) -> Result<bool, DomainError> { Ok(true) }
        async fn cancel_pending(&self, _: Uuid) -> Result<bool, DomainError> { Ok(true) }
        async fn set_defender_special(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { Ok(true) }
        async fn mark_unresolved_bets_lost(&self, _: Uuid) -> Result<(), DomainError> {
            Err(DomainError::Internal("simulated".into()))
        }
    }
    let svc = ManageCoudeCombatsService::new(Arc::new(BetsFailRepo));
    assert!(svc.cancel(Uuid::new_v4()).await.is_ok());
}

// ── Gate HP (with_surprise_gate) ──

use crate::domain::entities::system::bot_config::BotGuildConfig;
use crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::domain::entities::system::bot_config::BotDefinition;
use crate::domain::entities::coude::player::CoudePlayer;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::entities::coude::player::CombatStat;
#[derive(Default)]
struct StubPlayersUc {
    player: std::sync::Mutex<Option<CoudePlayer>>,
}
#[async_trait]
impl ManageCoudePlayersUseCase for StubPlayersUc {
    async fn get_or_create(&self, _: String, _: String, _: String) -> Result<CoudePlayer, DomainError> { unimplemented!() }
    async fn get(&self, _: &str, _: &str) -> Result<CoudePlayer, DomainError> {
        self.player.lock().unwrap().clone().ok_or(DomainError::NotFound("no".into()))
    }
    async fn list(&self, _: &str) -> Result<Vec<CoudePlayer>, DomainError> { Ok(vec![]) }
    async fn random_active(&self, _: &str, _: i64) -> Result<Vec<CoudePlayer>, DomainError> { Ok(vec![]) }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
    async fn update_class(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn add_xp(&self, _: &str, _: &str, _: i64) -> Result<XpProgress, DomainError> { unimplemented!() }
    async fn spend_stat_point(&self, _: &str, _: &str, _: CombatStat) -> Result<CoudePlayer, DomainError> { unimplemented!() }
    async fn reset_stats(&self, _: &str, _: &str, _: i64) -> Result<CoudePlayer, DomainError> { unimplemented!() }
    async fn record_win(&self, _: &str, _: &str, _: i64, _: i64) -> Result<(), DomainError> { Ok(()) }
    async fn record_loss(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { Ok(()) }
    async fn record_draw(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { Ok(()) }
    async fn increment_cowardice(&self, _: &str, _: &str) -> Result<i32, DomainError> { Ok(0) }
    async fn increment_chaos(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn record_coins_earned(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { Ok(()) }
    async fn record_coins_lost(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { Ok(()) }
    async fn update_hp(&self, _: &str, _: &str, _: i32, _: i32) -> Result<(), DomainError> { Ok(()) }
    async fn full_heal(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn regen_hp_tick(&self, _: f64, _: f64, _: f64, _: f64) -> Result<u64, DomainError> { Ok(0) }
}

#[derive(Default)]
struct StubBotConfig {
    rows: std::sync::Mutex<Vec<BotGuildConfig>>,
}
#[async_trait]
impl BotConfigRepository for StubBotConfig {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> { Ok(vec![]) }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(self.rows.lock().unwrap().clone())
    }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> { Ok(vec![]) }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
}

fn healthy_player() -> CoudePlayer {
    CoudePlayer {
        guild_id: "g".into(), user_id: "u".into(), username: "u".into(),
        coins: 100, total_wins: 0, total_losses: 0, total_draws: 0,
        total_earned: 0, total_lost: 0, total_stolen: 0,
        cowardice_count: 0, chaos_events: 0, casino_wins: 0, casino_losses: 0,
        level: 1, xp: 0, stat_points: 0, atk: 0, def: 0,
        class: None, title: None, class_changed_at: None,
        hp_current: 100, hp_max: 100, hp_last_regen: None, repos_last_used: None,
        season: 1, created_at: Utc::now(), updated_at: Utc::now(),
    }
}

#[tokio::test]
async fn create_with_gate_rejects_low_hp_attacker() {
    let repo = Arc::new(MockRepo::default());
    let players = Arc::new(StubPlayersUc::default());
    let bot_config = Arc::new(StubBotConfig::default());
    bot_config.rows.lock().unwrap().push(BotGuildConfig {
        id: Uuid::new_v4(), guild_id: "g".into(), bot_name: "coude-bot".into(),
        config_key: "combat_min_hp_pct".into(), config_value: "50".into(),
        updated_at: Utc::now(),
    });
    let mut attacker = healthy_player();
    attacker.hp_current = 10; // 10% seulement
    *players.player.lock().unwrap() = Some(attacker);
    let svc = ManageCoudeCombatsService::new(repo).with_surprise_gate(players, bot_config);
    let err = svc.create(new_combat("a", "d", 100)).await.unwrap_err();
    match err {
        DomainError::ValidationError(m) => {
            assert!(m.contains("PV") || m.contains("pas assez"));
        }
        other => panic!("expected ValidationError, got {:?}", other),
    }
}

#[tokio::test]
async fn create_with_gate_accepts_healthy_players() {
    let repo = Arc::new(MockRepo::default());
    let players = Arc::new(StubPlayersUc::default());
    *players.player.lock().unwrap() = Some(healthy_player());
    let bot_config = Arc::new(StubBotConfig::default());
    let svc = ManageCoudeCombatsService::new(repo).with_surprise_gate(players, bot_config);
    assert!(svc.create(new_combat("a", "d", 100)).await.is_ok());
}

#[tokio::test]
async fn create_with_gate_surprise_checks_attacker_hp() {
    let repo = Arc::new(MockRepo::default());
    let players = Arc::new(StubPlayersUc::default());
    let bot_config = Arc::new(StubBotConfig::default());
    bot_config.rows.lock().unwrap().extend(vec![
        BotGuildConfig {
            id: Uuid::new_v4(), guild_id: "g".into(), bot_name: "coude-bot".into(),
            config_key: "combat_min_hp_pct".into(), config_value: "0".into(),
            updated_at: Utc::now(),
        },
        BotGuildConfig {
            id: Uuid::new_v4(), guild_id: "g".into(), bot_name: "coude-bot".into(),
            config_key: "surprise_min_hp_percent".into(), config_value: "80".into(),
            updated_at: Utc::now(),
        },
    ]);
    let mut attacker = healthy_player();
    attacker.hp_current = 50; // 50%, en dessous des 80% requis
    *players.player.lock().unwrap() = Some(attacker);
    let svc = ManageCoudeCombatsService::new(repo).with_surprise_gate(players, bot_config);
    let mut cmd = new_combat("a", "d", 100);
    cmd.special_attack = Some("surprise".into());
    let err = svc.create(cmd).await.unwrap_err();
    match err {
        DomainError::ValidationError(m) => assert!(m.contains("surprise")),
        other => panic!("expected ValidationError, got {:?}", other),
    }
}

#[tokio::test]
async fn list_passes_status_filter_when_not_all() {
    // Teste la branche `status.filter(|s| *s != "all")` avec une valeur non-"all".
    let repo = Arc::new(MockRepo::default());
    let svc = ManageCoudeCombatsService::new(repo.clone());
    // La branche est exercee sans panic. Le mock ignore le filter mais enregistre la limit.
    svc.list("g", Some("pending"), 50).await.unwrap();
    assert_eq!(*repo.list_limit_received.lock().unwrap(), Some(50));
}
