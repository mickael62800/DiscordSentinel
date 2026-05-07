use super::*;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use sentinel_core::domain::entities::coude::bet::BetPayout;
use sentinel_core::domain::entities::coude::bet::BetResolutionPlan;
use sentinel_core::domain::entities::coude::bet::Bet;
use sentinel_core::domain::entities::coude::bet::NewCoudeBet;
use sentinel_core::domain::entities::coude::bet::RefundSummary;
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_bets::ManageCoudeBetsUseCase;
use crate::ports::inbound::coude::manage_bets::PlaceBetOutcome;
use crate::ports::inbound::coude::manage_bets::ResolveBetsOutcome;
#[derive(Default)]
struct MockBetsUc {
    placed: Mutex<Vec<NewCoudeBet>>,
    list_returns: Mutex<Vec<Bet>>,
    resolve_calls: Mutex<Vec<(Uuid, Option<String>)>>,
    refund_calls: Mutex<Vec<Uuid>>,
}

#[async_trait]
impl ManageCoudeBetsUseCase for MockBetsUc {
    async fn place(&self, new: NewCoudeBet) -> Result<PlaceBetOutcome, DomainError> {
        self.placed.lock().unwrap().push(new);
        Ok(PlaceBetOutcome { taunt_events: vec![] })
    }
    async fn list_for_combat(&self, _: Uuid) -> Result<Vec<Bet>, DomainError> {
        Ok(self.list_returns.lock().unwrap().clone())
    }
    async fn resolve(&self, id: Uuid, winner: Option<String>) -> Result<ResolveBetsOutcome, DomainError> {
        self.resolve_calls.lock().unwrap().push((id, winner));
        Ok(ResolveBetsOutcome {
            plan: BetResolutionPlan {
                payouts: vec![BetPayout {
                    bet_id: Uuid::from_u128(1),
                    bettor_id: "u".into(), bettor_name: "Alice".into(),
                    backed_id: "a".into(),
                    amount_bet: 100, payout: 250, won: true,
                }],
                fighter_bonus: None,
            },
            taunt_events: vec![],
        })
    }
    async fn refund(&self, id: Uuid) -> Result<RefundSummary, DomainError> {
        self.refund_calls.lock().unwrap().push(id);
        Ok(RefundSummary { refunded_count: 3, refunded_total: 750 })
    }
}

fn grpc(uc: Arc<MockBetsUc>) -> BetsGrpc {
    BetsGrpc { uc }
}

// ── place ──

#[tokio::test]
async fn place_delegates_to_uc() {
    let uc = Arc::new(MockBetsUc::default());
    let g = grpc(uc.clone());
    let combat_id = Uuid::new_v4();
    let resp = g.place(Request::new(proto::PlaceBetRequest {
        guild_id: "g1".into(),
        combat_id: combat_id.to_string(),
        bettor_id: "u1".into(),
        bettor_name: "Alice".into(),
        backed_id: "a1".into(),
        amount: 100,
    })).await.unwrap();
    assert!(resp.into_inner().taunt_events.is_empty());

    let placed = uc.placed.lock().unwrap();
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].combat_id, combat_id);
    assert_eq!(placed[0].amount, 100);
    assert_eq!(placed[0].bettor_name, "Alice");
}

#[tokio::test]
async fn place_invalid_uuid_returns_invalid_argument() {
    let g = grpc(Arc::new(MockBetsUc::default()));
    let err = g.place(Request::new(proto::PlaceBetRequest {
        guild_id: "g".into(),
        combat_id: "not-a-uuid".into(),
        bettor_id: "u".into(), bettor_name: "x".into(),
        backed_id: "a".into(), amount: 10,
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

// ── list_for_combat ──

#[tokio::test]
async fn list_for_combat_returns_bets_from_uc() {
    let uc = Arc::new(MockBetsUc::default());
    uc.list_returns.lock().unwrap().push(Bet {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        combat_id: Uuid::new_v4(),
        bettor_id: "u".into(),
        bettor_name: "Alice".into(),
        backed_id: "a".into(),
        amount: 100,
        won: None, payout: None,
    });
    let g = grpc(uc);
    let resp = g.list_for_combat(Request::new(proto::ListForCombatRequest {
        combat_id: Uuid::new_v4().to_string(),
    })).await.unwrap();
    assert_eq!(resp.into_inner().bets.len(), 1);
}

#[tokio::test]
async fn list_for_combat_invalid_uuid_rejected() {
    let g = grpc(Arc::new(MockBetsUc::default()));
    let err = g.list_for_combat(Request::new(proto::ListForCombatRequest {
        combat_id: "bad".into(),
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

// ── resolve ──

#[tokio::test]
async fn resolve_with_winner_delegates_to_uc() {
    let uc = Arc::new(MockBetsUc::default());
    let g = grpc(uc.clone());
    let combat_id = Uuid::new_v4();
    let resp = g.resolve(Request::new(proto::ResolveBetsRequest {
        combat_id: combat_id.to_string(),
        winner_id: Some("winner".into()),
    })).await.unwrap();
    let inner = resp.into_inner();
    assert!(inner.plan.is_some());
    assert_eq!(inner.plan.unwrap().payouts.len(), 1);
    let calls = uc.resolve_calls.lock().unwrap();
    assert_eq!(calls[0].0, combat_id);
    assert_eq!(calls[0].1, Some("winner".into()));
}

#[tokio::test]
async fn resolve_without_winner_is_draw() {
    let uc = Arc::new(MockBetsUc::default());
    let g = grpc(uc.clone());
    let _ = g.resolve(Request::new(proto::ResolveBetsRequest {
        combat_id: Uuid::new_v4().to_string(),
        winner_id: None,
    })).await.unwrap();
    let calls = uc.resolve_calls.lock().unwrap();
    assert_eq!(calls[0].1, None);
}

// ── refund ──

#[tokio::test]
async fn refund_returns_summary() {
    let uc = Arc::new(MockBetsUc::default());
    let g = grpc(uc.clone());
    let combat_id = Uuid::new_v4();
    let resp = g.refund(Request::new(proto::RefundBetsRequest {
        combat_id: combat_id.to_string(),
    })).await.unwrap();
    let summary = resp.into_inner();
    assert_eq!(summary.refunded_count, 3);
    assert_eq!(summary.refunded_total, 750);
    assert_eq!(uc.refund_calls.lock().unwrap()[0], combat_id);
}

#[tokio::test]
async fn refund_invalid_uuid_rejected() {
    let g = grpc(Arc::new(MockBetsUc::default()));
    let err = g.refund(Request::new(proto::RefundBetsRequest {
        combat_id: "garbage".into(),
    })).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}
