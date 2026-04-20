use super::*;
use crate::domain::entities::{BetResolutionPlan, CoudeBet, CoudeCombat, NewCoudeBet, RefundSummary, TauntEvent};
use crate::ports::inbound::manage_coude_bets::ManageCoudeBetsUseCase;
use crate::ports::inbound::manage_coude_combats::ManageCoudeCombatsUseCase;
use crate::ports::outbound::CoudeBetRepository;
use chrono::Utc;
use std::sync::Mutex as StdMutex;
use uuid::Uuid;

struct MockBetRepo {
    placed: StdMutex<Vec<NewCoudeBet>>,
}
impl MockBetRepo {
    fn new() -> Self { Self { placed: StdMutex::new(vec![]) } }
}

#[async_trait]
impl CoudeBetRepository for MockBetRepo {
    async fn list_for_combat(&self, _: Uuid) -> Result<Vec<CoudeBet>, DomainError> { Ok(vec![]) }
    async fn place(&self, new: NewCoudeBet) -> Result<Vec<TauntEvent>, DomainError> {
        self.placed.lock().unwrap().push(new);
        Ok(vec![])
    }
    async fn apply_resolution(&self, _: &str, _: BetResolutionPlan) -> Result<Vec<TauntEvent>, DomainError> { Ok(vec![]) }
    async fn refund_unresolved(&self, _: &str, _: Uuid) -> Result<RefundSummary, DomainError> {
        Ok(RefundSummary { refunded_count: 0, refunded_total: 0 })
    }
}

struct MockCombatsUc {
    status: String,
    attacker_id: String,
    defender_id: String,
    should_fail: bool,
}

impl MockCombatsUc {
    fn new(status: &str) -> Self {
        Self { status: status.into(), attacker_id: "att".into(), defender_id: "def".into(), should_fail: false }
    }
    fn failing() -> Self {
        Self { status: "betting".into(), attacker_id: "att".into(), defender_id: "def".into(), should_fail: true }
    }
}

#[async_trait]
impl ManageCoudeCombatsUseCase for MockCombatsUc {
    async fn list(&self, _: &str, _: Option<&str>, _: i64) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
    async fn get(&self, id: Uuid) -> Result<CoudeCombat, DomainError> {
        if self.should_fail {
            return Err(DomainError::NotFound("combat introuvable".into()));
        }
        Ok(CoudeCombat {
            id,
            guild_id: "g".into(),
            channel_id: None,
            attacker_id: self.attacker_id.clone(),
            attacker_name: "A".into(),
            defender_id: self.defender_id.clone(),
            defender_name: "D".into(),
            mise: 100,
            status: self.status.clone(),
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
        })
    }
    async fn get_pending_for_attacker(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
    async fn get_pending_for_defender(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
    async fn list_expired_pending(&self) -> Result<Vec<CoudeCombat>, DomainError> { Ok(vec![]) }
    async fn get_betting_for_participant(&self, _: &str, _: &str) -> Result<Option<CoudeCombat>, DomainError> { Ok(None) }
    async fn create(&self, _: crate::domain::entities::NewCoudeCombat) -> Result<CoudeCombat, DomainError> { unimplemented!() }
    async fn cancel(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
    async fn resolve(&self, _: Uuid, _: crate::domain::entities::CombatResolution) -> Result<(), DomainError> { Ok(()) }
    async fn set_betting(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn expire(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
    async fn set_defender_special(&self, _: Uuid, _: &str) -> Result<(), DomainError> { Ok(()) }
}

fn new_bet(bettor: &str, amount: i64) -> NewCoudeBet {
    NewCoudeBet {
        guild_id: "g".into(),
        combat_id: Uuid::new_v4(),
        bettor_id: bettor.into(),
        bettor_name: bettor.into(),
        backed_id: "att".into(),
        amount,
    }
}

fn make_svc(status: &str) -> (ManageCoudeBetsService, Arc<MockBetRepo>) {
    let repo = Arc::new(MockBetRepo::new());
    let combats = Arc::new(MockCombatsUc::new(status));
    let svc = ManageCoudeBetsService::new(repo.clone(), combats);
    (svc, repo)
}

#[tokio::test]
async fn place_rejects_zero_amount() {
    let (svc, _) = make_svc("betting");
    let err = svc.place(new_bet("u1", 0)).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn place_rejects_negative_amount() {
    let (svc, _) = make_svc("betting");
    let err = svc.place(new_bet("u1", -50)).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn place_rejects_combat_not_betting() {
    let (svc, repo) = make_svc("pending");
    let err = svc.place(new_bet("u1", 100)).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
    assert!(repo.placed.lock().unwrap().is_empty());
}

#[tokio::test]
async fn place_rejects_combat_resolved() {
    let (svc, _) = make_svc("resolved");
    assert!(matches!(svc.place(new_bet("u1", 100)).await, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn place_rejects_attacker_betting_on_own_combat() {
    let (svc, repo) = make_svc("betting");
    let err = svc.place(new_bet("att", 100)).await.unwrap_err();
    match err {
        DomainError::ValidationError(msg) => assert!(msg.contains("participant")),
        other => panic!("Expected ValidationError, got {:?}", other),
    }
    assert!(repo.placed.lock().unwrap().is_empty());
}

#[tokio::test]
async fn place_rejects_defender_betting_on_own_combat() {
    let (svc, _) = make_svc("betting");
    assert!(matches!(svc.place(new_bet("def", 100)).await, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn place_accepts_valid_bet() {
    let (svc, repo) = make_svc("betting");
    let outcome = svc.place(new_bet("u1", 100)).await.unwrap();
    let _ = outcome.taunt_events;
    assert_eq!(repo.placed.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn place_maps_combat_not_found_to_not_found() {
    let repo = Arc::new(MockBetRepo::new());
    let combats = Arc::new(MockCombatsUc::failing());
    let svc = ManageCoudeBetsService::new(repo, combats);
    let err = svc.place(new_bet("u1", 100)).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

// ── resolve() sans paris ──

#[tokio::test]
async fn resolve_empty_bets_returns_empty_plan() {
    let (svc, _) = make_svc("betting");
    let outcome = svc.resolve(Uuid::new_v4(), Some("att".into())).await.unwrap();
    assert!(outcome.plan.payouts.is_empty());
    assert!(outcome.taunt_events.is_empty());
}
