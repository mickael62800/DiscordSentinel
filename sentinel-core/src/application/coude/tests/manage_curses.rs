use super::*;

use std::sync::Mutex;

use chrono::Duration as ChronoDuration;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::casino::wallet::Wallet;
use crate::domain::entities::casino::wallet::WalletTransaction;
#[derive(Default)]
struct MockCursesRepo {
    actives: Mutex<Vec<ActiveCurse>>,
    cast_calls: Mutex<u32>,
    lift_calls: Mutex<u32>,
    fail_cast: Mutex<bool>,
}

#[async_trait]
impl CursesRepository for MockCursesRepo {
    async fn cast(
        &self,
        guild_id: &str,
        target_id: &str,
        source_id: &str,
        kind: CurseKind,
        duration_hours: i64,
    ) -> Result<Uuid, DomainError> {
        if *self.fail_cast.lock().unwrap() {
            return Err(DomainError::Conflict("test forced fail".into()));
        }
        *self.cast_calls.lock().unwrap() += 1;
        let id = Uuid::new_v4();
        self.actives.lock().unwrap().push(ActiveCurse {
            id,
            guild_id: guild_id.into(),
            target_id: target_id.into(),
            source_id: source_id.into(),
            kind,
            created_at: Utc::now(),
            expires_at: Utc::now() + ChronoDuration::hours(duration_hours),
            lifted_at: None,
            lifted_by: None,
            uses_remaining: None,
        });
        Ok(id)
    }

    async fn get_active_for_target(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveCurse>, DomainError> {
        Ok(self
            .actives
            .lock()
            .unwrap()
            .iter()
            .find(|c| {
                c.guild_id.as_str() == guild_id
                    && c.target_id == target_id
                    && c.lifted_at.is_none()
                    && c.expires_at > Utc::now()
            })
            .cloned())
    }

    async fn lift(&self, id: Uuid, lifted_by: &str) -> Result<(), DomainError> {
        let mut guard = self.actives.lock().unwrap();
        let entry = guard
            .iter_mut()
            .find(|c| c.id == id && c.lifted_at.is_none())
            .ok_or_else(|| DomainError::Conflict("introuvable".into()))?;
        entry.lifted_at = Some(Utc::now());
        entry.lifted_by = Some(lifted_by.into());
        *self.lift_calls.lock().unwrap() += 1;
        Ok(())
    }

    async fn list_active_by_source(
        &self,
        _guild_id: &str,
        _source_id: &str,
    ) -> Result<Vec<ActiveCurse>, DomainError> {
        Ok(vec![])
    }
}

#[derive(Default)]
struct SpyWalletRepo {
    debits: Mutex<Vec<(String, String, i64, String)>>,
    transfers: Mutex<Vec<(String, String, String, i64, String)>>,
    debit_should_fail: Mutex<bool>,
}

fn mk_wallet(g: &str, u: &str) -> Wallet {
    Wallet {
        id: Uuid::new_v4(),
        guild_id: g.into(),
        user_id: u.into(),
        username: "x".into(),
        coins: 1_000_000,
        total_earned: 0,
        total_spent: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[async_trait]
impl WalletRepository for SpyWalletRepo {
    async fn get_or_create(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Wallet, DomainError> {
        unimplemented!()
    }
    async fn get(&self, _: &str, _: &str) -> Result<Option<Wallet>, DomainError> {
        Ok(None)
    }
    async fn credit(
        &self,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<Wallet, DomainError> {
        unimplemented!()
    }
    async fn debit(
        &self,
        g: &str,
        u: &str,
        amount: i64,
        source: &str,
        _: &str,
    ) -> Result<Wallet, DomainError> {
        if *self.debit_should_fail.lock().unwrap() {
            return Err(DomainError::ValidationError("solde insuffisant".into()));
        }
        self.debits
            .lock()
            .unwrap()
            .push((g.into(), u.into(), amount, source.into()));
        Ok(mk_wallet(g, u))
    }
    async fn transfer(
        &self,
        g: &str,
        from: &str,
        to: &str,
        amount: i64,
        source: &str,
        _: &str,
    ) -> Result<(), DomainError> {
        self.transfers.lock().unwrap().push((
            g.into(),
            from.into(),
            to.into(),
            amount,
            source.into(),
        ));
        Ok(())
    }
    async fn pay_combat_atomic(
        &self,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn debit_pair_atomic(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn leaderboard(&self, _: &str, _: i64) -> Result<Vec<Wallet>, DomainError> {
        Ok(vec![])
    }
    async fn get_transactions(
        &self,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Vec<WalletTransaction>, DomainError> {
        Ok(vec![])
    }
    async fn list_by_guild(&self, _: &str) -> Result<Vec<Wallet>, DomainError> {
        Ok(vec![])
    }
    async fn reset_wallet(&self, _: &str, _: &str, _: i64) -> Result<Wallet, DomainError> {
        unimplemented!()
    }
    async fn reset_all_wallets(&self, _: &str, _: i64) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn credit_in_tx(
        &self,
        _: &mut dyn crate::ports::uow::DbTx,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<(i64, i64), DomainError> {
        unimplemented!()
    }
    async fn debit_in_tx(
        &self,
        _: &mut dyn crate::ports::uow::DbTx,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<(i64, i64), DomainError> {
        unimplemented!()
    }
}

fn make_service() -> (
    ManageCoudeCursesService,
    Arc<MockCursesRepo>,
    Arc<SpyWalletRepo>,
) {
    let curses = Arc::new(MockCursesRepo::default());
    let wallet = Arc::new(SpyWalletRepo::default());
    let svc = ManageCoudeCursesService::new(curses.clone(), wallet.clone());
    (svc, curses, wallet)
}

#[tokio::test]
async fn cast_self_is_rejected() {
    let (svc, _, _) = make_service();
    let err = svc
        .cast("g", "u", "name", "u", Some(CurseKind::Banana))
        .await
        .unwrap_err();
    matches!(err, DomainError::ValidationError(_));
}

#[tokio::test]
async fn cast_debits_300_and_inserts() {
    let (svc, curses, wallet) = make_service();
    let out = svc
        .cast("g", "src", "name", "tgt", Some(CurseKind::Banana))
        .await
        .unwrap();
    assert_eq!(out.kind, CurseKind::Banana);
    assert_eq!(out.cost_paid, 300);
    let debits = wallet.debits.lock().unwrap();
    assert_eq!(debits.len(), 1);
    assert_eq!(debits[0].2, 300);
    assert_eq!(*curses.cast_calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn cast_random_when_kind_omitted() {
    let (svc, _, _) = make_service();
    let out = svc.cast("g", "src", "name", "tgt", None).await.unwrap();
    // Le kind doit etre l un des 6.
    assert!(CurseKind::ALL.contains(&out.kind));
}

#[tokio::test]
async fn cast_rejected_when_active_curse_exists() {
    let (svc, _, _) = make_service();
    svc.cast("g", "src", "n", "tgt", Some(CurseKind::Banana))
        .await
        .unwrap();
    let err = svc
        .cast("g", "other", "n2", "tgt", Some(CurseKind::Chicken))
        .await
        .unwrap_err();
    matches!(err, DomainError::Conflict(_));
}

#[tokio::test]
async fn cast_no_debit_if_active_curse_exists() {
    let (svc, _, wallet) = make_service();
    svc.cast("g", "src", "n", "tgt", Some(CurseKind::Banana))
        .await
        .unwrap();
    let err = svc
        .cast("g", "other", "n2", "tgt", Some(CurseKind::Chicken))
        .await;
    assert!(err.is_err());
    let debits = wallet.debits.lock().unwrap();
    assert_eq!(debits.len(), 1, "seul le 1er cast doit avoir debite");
}

#[tokio::test]
async fn cast_propagates_wallet_failure() {
    let (svc, curses, wallet) = make_service();
    *wallet.debit_should_fail.lock().unwrap() = true;
    let err = svc
        .cast("g", "src", "n", "tgt", Some(CurseKind::Banana))
        .await
        .unwrap_err();
    matches!(err, DomainError::ValidationError(_));
    assert_eq!(
        *curses.cast_calls.lock().unwrap(),
        0,
        "pas d insert si debit a foire"
    );
}

#[tokio::test]
async fn lift_transfers_double_to_source() {
    let (svc, _, wallet) = make_service();
    svc.cast("g", "src", "n", "tgt", Some(CurseKind::Banana))
        .await
        .unwrap();
    let updated = svc.lift_own("g", "tgt", "tname").await.unwrap();
    assert!(updated.lifted_at.is_some());
    let transfers = wallet.transfers.lock().unwrap();
    assert_eq!(transfers.len(), 1);
    let (_, from, to, amount, _) = &transfers[0];
    assert_eq!(from, "tgt");
    assert_eq!(to, "src");
    assert_eq!(*amount, 600);
}

#[tokio::test]
async fn lift_when_no_curse_returns_not_found() {
    let (svc, _, _) = make_service();
    let err = svc.lift_own("g", "tgt", "tname").await.unwrap_err();
    matches!(err, DomainError::NotFound(_));
}

#[tokio::test]
async fn get_active_returns_curse_when_present() {
    let (svc, _, _) = make_service();
    svc.cast("g", "src", "n", "tgt", Some(CurseKind::Insomnia))
        .await
        .unwrap();
    let got = svc.get_active("g", "tgt").await.unwrap();
    assert!(got.is_some());
    assert_eq!(got.unwrap().kind, CurseKind::Insomnia);
}

#[tokio::test]
async fn get_active_returns_none_when_absent() {
    let (svc, _, _) = make_service();
    assert!(svc.get_active("g", "tgt").await.unwrap().is_none());
}
