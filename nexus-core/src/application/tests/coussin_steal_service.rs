use std::sync::{Arc, Mutex};
use async_trait::async_trait;

use crate::domain::errors::DomainError;
use crate::ports::inbound::coussin_steal::CoussinStealUseCase;
use crate::ports::outbound::coussin_steal_repository::CoussinStealRepository;
use crate::application::coussin_steal_service::CoussinStealService;
use crate::application::economy_config::EmptyBotConfigRepository;

#[derive(Default)]
struct MockStealRepo {
    thief_balance: i64,
    victim_balance: i64,
    transferred: Mutex<Option<(i64, bool)>>,
}

#[async_trait]
impl CoussinStealRepository for MockStealRepo {
    async fn balances(&self, _g: &str, _t: &str, _v: &str) -> Result<(i64, i64), DomainError> {
        Ok((self.thief_balance, self.victim_balance))
    }
    async fn transfer(
        &self,
        _g: &str,
        _t: &str,
        _v: &str,
        amount: i64,
        success: bool,
        _cd: i64,
    ) -> Result<(), DomainError> {
        *self.transferred.lock().unwrap() = Some((amount, success));
        Ok(())
    }
}

#[tokio::test]
async fn test_cannot_steal_self() {
    let service = CoussinStealService::new(
        Arc::new(MockStealRepo::default()),
        Arc::new(EmptyBotConfigRepository),
    );
    let res = service.steal("g1", "u1", "u1", false).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_cannot_steal_poor_victim() {
    let repo = Arc::new(MockStealRepo {
        thief_balance: 500,
        victim_balance: 5, // Below default 10 min
        ..Default::default()
    });
    let service = CoussinStealService::new(repo, Arc::new(EmptyBotConfigRepository));
    let res = service.steal("g1", "u1", "u2", false).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_steal_executes_transfer() {
    let repo = Arc::new(MockStealRepo {
        thief_balance: 500,
        victim_balance: 500,
        ..Default::default()
    });
    let service = CoussinStealService::new(repo.clone(), Arc::new(EmptyBotConfigRepository));
    let res = service.steal("g1", "u1", "u2", false).await;
    assert!(res.is_ok());
    assert!(repo.transferred.lock().unwrap().is_some());
}
