use super::*;
use crate::ports::inbound::coude::manage_economy::ManageCoudeEconomyUseCase;
use crate::ports::inbound::coude::manage_economy::StealOutcome;
use async_trait::async_trait;
use sentinel_core::domain::entities::coude::taunt::TauntEvent;
use sentinel_core::domain::errors::DomainError;
use std::sync::Arc;
use std::sync::Mutex;
#[derive(Default)]
struct MockEconomyUc {
    transfer_calls: Mutex<Vec<(String, String, String, i64)>>,
    steal_calls: Mutex<Vec<(String, String, String, i64)>>,
    steal_fail_calls: Mutex<Vec<(String, String, i64)>>,
    casino_win_calls: Mutex<Vec<(String, String, i64)>>,
    casino_loss_calls: Mutex<Vec<(String, String, i64)>>,
    faillite_calls: Mutex<Vec<(String, String)>>,
    faillite_return: Mutex<i64>,
    count_return: Mutex<i64>,
    sum_return: Mutex<i64>,
}

#[async_trait]
impl ManageCoudeEconomyUseCase for MockEconomyUc {
    async fn transfer(
        &self,
        g: &str,
        f: &str,
        t: &str,
        a: i64,
    ) -> Result<Vec<TauntEvent>, DomainError> {
        self.transfer_calls
            .lock()
            .unwrap()
            .push((g.into(), f.into(), t.into(), a));
        Ok(vec![])
    }
    async fn steal(&self, g: &str, th: &str, v: &str, a: i64) -> Result<StealOutcome, DomainError> {
        self.steal_calls
            .lock()
            .unwrap()
            .push((g.into(), th.into(), v.into(), a));
        Ok(StealOutcome {
            stolen: a.min(100),
            taunt_events: vec![],
        })
    }
    async fn steal_fail_penalty(
        &self,
        g: &str,
        t: &str,
        a: i64,
    ) -> Result<(i64, Vec<TauntEvent>), DomainError> {
        self.steal_fail_calls
            .lock()
            .unwrap()
            .push((g.into(), t.into(), a));
        Ok((a, vec![]))
    }
    async fn record_casino_win(&self, g: &str, u: &str, gain: i64) -> Result<(), DomainError> {
        self.casino_win_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), gain));
        Ok(())
    }
    async fn record_casino_loss(&self, g: &str, u: &str, l: i64) -> Result<(), DomainError> {
        self.casino_loss_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into(), l));
        Ok(())
    }
    async fn record_casino_faillite(&self, g: &str, u: &str) -> Result<i64, DomainError> {
        self.faillite_calls
            .lock()
            .unwrap()
            .push((g.into(), u.into()));
        Ok(*self.faillite_return.lock().unwrap())
    }
    async fn count_casino_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(*self.count_return.lock().unwrap())
    }
    async fn sum_casino_gains_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(*self.sum_return.lock().unwrap())
    }
    async fn count_steal_today(&self, _: &str, _: &str) -> Result<i64, DomainError> {
        Ok(*self.count_return.lock().unwrap())
    }
}

fn grpc(uc: Arc<MockEconomyUc>) -> EconomyGrpc {
    EconomyGrpc { uc }
}

// ── transfer ──

#[tokio::test]
async fn transfer_delegates_to_uc() {
    let uc = Arc::new(MockEconomyUc::default());
    let g = grpc(uc.clone());
    let resp = g
        .transfer(Request::new(proto::TransferRequest {
            guild_id: "g1".into(),
            from_id: "from".into(),
            to_id: "to".into(),
            amount: 500,
        }))
        .await
        .unwrap();
    assert!(resp.into_inner().taunt_events.is_empty());

    let calls = uc.transfer_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], ("g1".into(), "from".into(), "to".into(), 500));
}

// ── steal ──

#[tokio::test]
async fn steal_delegates_and_maps_outcome() {
    let uc = Arc::new(MockEconomyUc::default());
    let g = grpc(uc.clone());
    let resp = g
        .steal(Request::new(proto::StealRequest {
            guild_id: "g1".into(),
            thief_id: "t1".into(),
            victim_id: "v1".into(),
            amount: 1000,
        }))
        .await
        .unwrap();
    // MockEconomyUc.steal clamp a 100
    assert_eq!(resp.into_inner().stolen, 100);
    assert_eq!(uc.steal_calls.lock().unwrap().len(), 1);
}

// ── steal_fail_penalty ──

#[tokio::test]
async fn steal_fail_penalty_returns_lost_amount() {
    let uc = Arc::new(MockEconomyUc::default());
    let g = grpc(uc.clone());
    let resp = g
        .steal_fail_penalty(Request::new(proto::StealFailPenaltyRequest {
            guild_id: "g".into(),
            thief_id: "t".into(),
            amount: 250,
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().lost, 250);
    assert_eq!(
        uc.steal_fail_calls.lock().unwrap()[0],
        ("g".into(), "t".into(), 250)
    );
}

// ── record_casino_win/loss ──

#[tokio::test]
async fn record_casino_win_delegates() {
    let uc = Arc::new(MockEconomyUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .record_casino_win(Request::new(proto::RecordCasinoWinRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            gain: 1500,
        }))
        .await
        .unwrap();
    assert_eq!(
        uc.casino_win_calls.lock().unwrap()[0],
        ("g".into(), "u".into(), 1500)
    );
}

#[tokio::test]
async fn record_casino_loss_delegates() {
    let uc = Arc::new(MockEconomyUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .record_casino_loss(Request::new(proto::RecordCasinoLossRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            lost: 300,
        }))
        .await
        .unwrap();
    assert_eq!(
        uc.casino_loss_calls.lock().unwrap()[0],
        ("g".into(), "u".into(), 300)
    );
}

#[tokio::test]
async fn record_casino_faillite_returns_cleared_coins() {
    let uc = Arc::new(MockEconomyUc::default());
    *uc.faillite_return.lock().unwrap() = 999;
    let g = grpc(uc.clone());
    let resp = g
        .record_casino_faillite(Request::new(proto::RecordCasinoFailliteRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().cleared_coins, 999);
}

// ── Counters (Int64Value) ──

#[tokio::test]
async fn count_casino_today_returns_value() {
    let uc = Arc::new(MockEconomyUc::default());
    *uc.count_return.lock().unwrap() = 7;
    let g = grpc(uc.clone());
    let resp = g
        .count_casino_today(Request::new(proto::UserInGuildRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().value, 7);
}

#[tokio::test]
async fn sum_casino_gains_today_returns_value() {
    let uc = Arc::new(MockEconomyUc::default());
    *uc.sum_return.lock().unwrap() = 12_000;
    let g = grpc(uc.clone());
    let resp = g
        .sum_casino_gains_today(Request::new(proto::UserInGuildRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().value, 12_000);
}

#[tokio::test]
async fn count_steal_today_returns_value() {
    let uc = Arc::new(MockEconomyUc::default());
    *uc.count_return.lock().unwrap() = 3;
    let g = grpc(uc.clone());
    let resp = g
        .count_steal_today(Request::new(proto::UserInGuildRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().value, 3);
}
