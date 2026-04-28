use super::*;
use crate::domain::entities::{BetResolutionPlan, CoudeBet, CoudeCombat, NewCoudeBet, RefundSummary, TauntEvent};
use crate::ports::inbound::manage_bets::ManageCoudeBetsUseCase;
use crate::ports::outbound::{CombatQueryRepository, CoudeBetRepository};
use chrono::Utc;
use std::sync::Mutex as StdMutex;
use uuid::Uuid;

struct MockBetRepo {
    placed: StdMutex<Vec<NewCoudeBet>>,
    bets: StdMutex<Vec<CoudeBet>>,
    apply_calls: StdMutex<Vec<(String, BetResolutionPlan)>>,
    refund_summary: StdMutex<RefundSummary>,
    refund_calls: StdMutex<Vec<(String, Uuid)>>,
}
impl MockBetRepo {
    fn new() -> Self {
        Self {
            placed: StdMutex::new(vec![]),
            bets: StdMutex::new(vec![]),
            apply_calls: StdMutex::new(vec![]),
            refund_summary: StdMutex::new(RefundSummary { refunded_count: 0, refunded_total: 0 }),
            refund_calls: StdMutex::new(vec![]),
        }
    }
}

#[async_trait]
impl CoudeBetRepository for MockBetRepo {
    async fn list_for_combat(&self, _: Uuid) -> Result<Vec<CoudeBet>, DomainError> {
        Ok(self.bets.lock().unwrap().clone())
    }
    async fn place(&self, new: NewCoudeBet) -> Result<Vec<TauntEvent>, DomainError> {
        self.placed.lock().unwrap().push(new);
        Ok(vec![])
    }
    async fn apply_resolution(&self, g: &str, p: BetResolutionPlan) -> Result<Vec<TauntEvent>, DomainError> {
        self.apply_calls.lock().unwrap().push((g.into(), p));
        Ok(vec![])
    }
    async fn refund_unresolved(&self, g: &str, id: Uuid) -> Result<RefundSummary, DomainError> {
        self.refund_calls.lock().unwrap().push((g.into(), id));
        Ok(self.refund_summary.lock().unwrap().clone())
    }
}

struct MockCombatQuery {
    status: String,
    attacker_id: String,
    defender_id: String,
    should_fail: bool,
}

impl MockCombatQuery {
    fn new(status: &str) -> Self {
        Self { status: status.into(), attacker_id: "att".into(), defender_id: "def".into(), should_fail: false }
    }
    fn failing() -> Self {
        Self { status: "betting".into(), attacker_id: "att".into(), defender_id: "def".into(), should_fail: true }
    }
}

#[async_trait]
impl CombatQueryRepository for MockCombatQuery {
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
    let combats = Arc::new(MockCombatQuery::new(status));
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
    let combats = Arc::new(MockCombatQuery::failing());
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

// ── list_for_combat / refund ──

#[tokio::test]
async fn list_for_combat_delegates_repo() {
    let (svc, repo) = make_svc("betting");
    repo.bets.lock().unwrap().push(CoudeBet {
        id: Uuid::new_v4(), guild_id: "g".into(), combat_id: Uuid::new_v4(),
        bettor_id: "u1".into(), bettor_name: "U1".into(),
        backed_id: "att".into(), amount: 100,
        won: None, payout: None,
    });
    let bets = svc.list_for_combat(Uuid::new_v4()).await.unwrap();
    assert_eq!(bets.len(), 1);
    assert_eq!(bets[0].amount, 100);
}

#[tokio::test]
async fn refund_delegates_to_repo_with_guild_id() {
    let (svc, repo) = make_svc("betting");
    *repo.refund_summary.lock().unwrap() = RefundSummary { refunded_count: 3, refunded_total: 900 };
    let combat_id = Uuid::new_v4();
    let summary = svc.refund(combat_id).await.unwrap();
    assert_eq!(summary.refunded_count, 3);
    assert_eq!(summary.refunded_total, 900);
    let calls = repo.refund_calls.lock().unwrap();
    assert_eq!(calls[0].0, "g");
    assert_eq!(calls[0].1, combat_id);
}

#[tokio::test]
async fn resolve_with_bets_invokes_apply_resolution() {
    let (svc, repo) = make_svc("betting");
    let combat_id = Uuid::new_v4();
    // Un pari sur l'attaquant pour qu'un payout soit calcule.
    repo.bets.lock().unwrap().push(CoudeBet {
        id: Uuid::new_v4(), guild_id: "g".into(), combat_id,
        bettor_id: "u1".into(), bettor_name: "U1".into(),
        backed_id: "att".into(), amount: 100,
        won: None, payout: None,
    });
    let _outcome = svc.resolve(combat_id, Some("att".into())).await.unwrap();
    assert_eq!(repo.apply_calls.lock().unwrap().len(), 1);
    assert_eq!(repo.apply_calls.lock().unwrap()[0].0, "g");
}

#[tokio::test]
async fn refund_propagates_combat_not_found() {
    let repo = Arc::new(MockBetRepo::new());
    let combats = Arc::new(MockCombatQuery::failing());
    let svc = ManageCoudeBetsService::new(repo, combats);
    assert!(matches!(svc.refund(Uuid::new_v4()).await, Err(DomainError::NotFound(_))));
}
