use super::*;
use crate::domain::entities::{CombatResolution, CoudeCombat, NewCoudeCombat};
use crate::ports::inbound::manage_coude_combats::ManageCoudeCombatsUseCase;
use crate::ports::outbound::CoudeCombatRepository;
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
impl CoudeCombatRepository for MockRepo {
    async fn list(&self, _: &str, _: Option<&str>, limit: i64) -> Result<Vec<CoudeCombat>, DomainError> {
        *self.list_limit_received.lock().unwrap() = Some(limit);
        Ok(vec![])
    }
    async fn get(&self, _: Uuid) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
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
async fn create_accepts_zero_mise() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    assert!(svc.create(new_combat("a", "d", 0)).await.is_ok());
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

#[tokio::test]
async fn set_defender_special_rejects_empty_item() {
    let svc = ManageCoudeCombatsService::new(Arc::new(MockRepo::default()));
    assert!(matches!(svc.set_defender_special(Uuid::new_v4(), "").await, Err(DomainError::ValidationError(_))));
}
