//! Tests du service Roue du Destin — focus sur les chemins de validation
//! qui s executent AVANT toute tx DB. Les mutations tx sont mockees
//! `unimplemented!()` (couvertes par tests d integration Postgres).

use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::application::casino::manage_wheel_service::ManageWheelService;
use crate::domain::entities::casino::wheel::WheelSpin;
use crate::domain::entities::casino::wheel::WheelTopWinner;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::casino::manage_wallet::TxWalletMutation;
use crate::ports::inbound::casino::manage_wallet::WalletMutation;
use crate::ports::inbound::casino::manage_wheel::ManageWheelUseCase;
use crate::ports::inbound::casino::manage_wheel::WheelSpinCommand;
use crate::ports::outbound::casino::wheel_repository::WheelRepository;

#[derive(Default)]
struct MockWheelRepo {
    has_claimed: StdMutex<bool>,
    recent_returns: StdMutex<Vec<WheelSpin>>,
    top_returns: StdMutex<Vec<WheelTopWinner>>,
}

#[async_trait]
impl WheelRepository for MockWheelRepo {
    async fn has_claimed_today(&self, _g: &str, _u: &str) -> Result<bool, DomainError> {
        Ok(*self.has_claimed.lock().unwrap())
    }
    async fn log_spin_in_tx(
        &self,
        _: &mut dyn crate::ports::uow::DbTx,
        _: &WheelSpin,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn mark_claimed_in_tx(
        &self,
        _: &mut dyn crate::ports::uow::DbTx,
        _: &str,
        _: &str,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn recent_spins(&self, _g: &str, _l: i64) -> Result<Vec<WheelSpin>, DomainError> {
        Ok(self.recent_returns.lock().unwrap().clone())
    }
    async fn top_winners(
        &self,
        _g: &str,
        _d: i64,
        _l: i64,
    ) -> Result<Vec<WheelTopWinner>, DomainError> {
        Ok(self.top_returns.lock().unwrap().clone())
    }
}

struct MockWalletUc;

#[async_trait]
impl ManageWalletUseCase for MockWalletUc {
    async fn credit(
        &self,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<WalletMutation, DomainError> {
        unimplemented!()
    }
    async fn debit(
        &self,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<WalletMutation, DomainError> {
        unimplemented!()
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
        unimplemented!()
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

// UnitOfWork factice : begin() echoue, ce qui permet aux tests de valider les
// chemins business AVANT toute tx DB et de s'arreter sur une erreur non-validation.
struct MockUow;
#[async_trait]
impl crate::ports::uow::UnitOfWork for MockUow {
    async fn begin(&self) -> Result<Box<dyn crate::ports::uow::DbTx>, DomainError> {
        Err(DomainError::Internal("no db in tests".into()))
    }
    async fn commit(&self, _tx: Box<dyn crate::ports::uow::DbTx>) -> Result<(), DomainError> {
        Err(DomainError::Internal("no db in tests".into()))
    }
    async fn rollback(&self, _tx: Box<dyn crate::ports::uow::DbTx>) -> Result<(), DomainError> {
        Ok(())
    }
}

fn make_service(repo: Arc<MockWheelRepo>) -> ManageWheelService {
    ManageWheelService::new(repo, Arc::new(MockWalletUc), Arc::new(MockUow))
}

fn cmd() -> WheelSpinCommand {
    WheelSpinCommand {
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "Alice".into(),
    }
}

// ══════════════════════════════════════════════════════════
// Validation
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn spin_rejects_when_already_claimed_today() {
    let repo = MockWheelRepo::default();
    *repo.has_claimed.lock().unwrap() = true;
    let svc = make_service(Arc::new(repo));
    let err = svc.spin(cmd()).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(m) if m.contains("deja tire")));
}

#[tokio::test]
async fn spin_passes_validation_when_not_claimed() {
    // Sans claim, on devrait passer la validation et echouer plus tard sur la tx
    let svc = make_service(Arc::new(MockWheelRepo::default()));
    let err = svc.spin(cmd()).await.unwrap_err();
    assert!(!matches!(err, DomainError::ValidationError(m) if m.contains("deja tire")));
}

// ══════════════════════════════════════════════════════════
// Read-only delegations
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn recent_spins_delegates() {
    let repo = MockWheelRepo::default();
    *repo.recent_returns.lock().unwrap() = vec![WheelSpin {
        id: Uuid::nil(),
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "Alice".into(),
        case_key: "jackpot".into(),
        case_label: "🎰 Jackpot".into(),
        payout: 5000,
        created_at: Utc::now(),
    }];
    let svc = make_service(Arc::new(repo));
    let out = svc.recent_spins("g", 10).await.unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].payout, 5000);
}

#[tokio::test]
async fn top_winners_delegates() {
    let repo = MockWheelRepo::default();
    *repo.top_returns.lock().unwrap() = vec![WheelTopWinner {
        user_id: "u1".into(),
        username: "Bob".into(),
        total_payout: 12500,
        spin_count: 7,
    }];
    let svc = make_service(Arc::new(repo));
    let out = svc.top_winners("g", 7, 10).await.unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].spin_count, 7);
}

#[tokio::test]
async fn empty_recent_spins() {
    let svc = make_service(Arc::new(MockWheelRepo::default()));
    let out = svc.recent_spins("g", 10).await.unwrap();
    assert!(out.is_empty());
}
