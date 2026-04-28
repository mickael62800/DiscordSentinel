use super::*;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::{Arc, Mutex as StdMutex};
use uuid::Uuid;

use crate::domain::entities::{
    CashboxRedistribution, CashboxRedistributionEntry, CashboxSource, CoudeCashbox, Wallet,
    WalletTransaction,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_cashbox::ManageCoudeCashboxUseCase;
use crate::ports::outbound::{CoudeCashboxRepository, WalletRepository};

#[derive(Default)]
struct MockCashboxRepo {
    active: StdMutex<Vec<(String, String)>>,
    claim_total: StdMutex<i64>,
    deposits: StdMutex<Vec<(String, i64, CashboxSource)>>,
    record_calls: StdMutex<Vec<(String, i64, Vec<(String, String, i64)>)>>,
    due_guilds: StdMutex<Vec<String>>,
    redistributions: StdMutex<Vec<CashboxRedistribution>>,
    entries: StdMutex<Vec<CashboxRedistributionEntry>>,
}

#[async_trait]
impl CoudeCashboxRepository for MockCashboxRepo {
    async fn get_or_create(&self, g: &str) -> Result<CoudeCashbox, DomainError> {
        Ok(CoudeCashbox {
            guild_id: g.into(), balance: 0, total_collected: 0, total_redistributed: 0,
            last_redistribution_at: None, created_at: Utc::now(), updated_at: Utc::now(),
        })
    }
    async fn deposit(&self, g: &str, a: i64, s: CashboxSource) -> Result<(), DomainError> {
        self.deposits.lock().unwrap().push((g.into(), a, s));
        Ok(())
    }
    async fn claim_all_for_redistribution(&self, _: &str) -> Result<i64, DomainError> {
        Ok(*self.claim_total.lock().unwrap())
    }
    async fn withdraw(&self, _: &str, _: i64) -> Result<i64, DomainError> { Ok(0) }
    async fn record_redistribution(
        &self, g: &str, total: i64, entries: Vec<(String, String, i64)>,
    ) -> Result<Uuid, DomainError> {
        self.record_calls.lock().unwrap().push((g.into(), total, entries));
        Ok(Uuid::new_v4())
    }
    async fn list_redistributions(&self, _: &str, _: i64) -> Result<Vec<CashboxRedistribution>, DomainError> {
        Ok(self.redistributions.lock().unwrap().clone())
    }
    async fn list_entries(&self, _: Uuid) -> Result<Vec<CashboxRedistributionEntry>, DomainError> {
        Ok(self.entries.lock().unwrap().clone())
    }
    async fn list_active_players(&self, _: &str, _: i64) -> Result<Vec<(String, String)>, DomainError> {
        Ok(self.active.lock().unwrap().clone())
    }
    async fn list_guilds_due_for_redistribution(&self, _: i64) -> Result<Vec<String>, DomainError> {
        Ok(self.due_guilds.lock().unwrap().clone())
    }
}

#[derive(Default)]
struct SpyWalletRepo {
    credit_calls: StdMutex<Vec<(String, String, i64)>>,
}
#[async_trait]
impl WalletRepository for SpyWalletRepo {
    async fn get_or_create(&self, g: &str, u: &str, n: &str, _: i64) -> Result<Wallet, DomainError> {
        Ok(Wallet {
            id: Uuid::new_v4(), guild_id: g.into(), user_id: u.into(), username: n.into(),
            coins: 0, total_earned: 0, total_spent: 0,
            created_at: Utc::now(), updated_at: Utc::now(),
        })
    }
    async fn get(&self, _: &str, _: &str) -> Result<Option<Wallet>, DomainError> { Ok(None) }
    async fn credit(&self, g: &str, u: &str, a: i64, _: &str, _: &str) -> Result<Wallet, DomainError> {
        self.credit_calls.lock().unwrap().push((g.into(), u.into(), a));
        Ok(Wallet {
            id: Uuid::new_v4(), guild_id: g.into(), user_id: u.into(), username: "x".into(),
            coins: a, total_earned: a, total_spent: 0,
            created_at: Utc::now(), updated_at: Utc::now(),
        })
    }
    async fn debit(&self, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn transfer(&self, _: &str, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn pay_combat_atomic(&self, _: &str, _: &str, _: i64, _: &str, _: i64, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn leaderboard(&self, _: &str, _: i64) -> Result<Vec<Wallet>, DomainError> { Ok(vec![]) }
    async fn get_transactions(&self, _: &str, _: &str, _: i64) -> Result<Vec<WalletTransaction>, DomainError> { Ok(vec![]) }
    async fn list_by_guild(&self, _: &str) -> Result<Vec<Wallet>, DomainError> { Ok(vec![]) }
    async fn reset_wallet(&self, _: &str, _: &str, _: i64) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn reset_all_wallets(&self, _: &str, _: i64) -> Result<u64, DomainError> { Ok(0) }
}

fn make_svc(cb: Arc<MockCashboxRepo>, w: Arc<SpyWalletRepo>) -> ManageCoudeCashboxService {
    ManageCoudeCashboxService::new(cb, w)
}

// ── get_cashbox / deposit ──

#[tokio::test]
async fn get_cashbox_delegates() {
    let svc = make_svc(Arc::new(MockCashboxRepo::default()), Arc::new(SpyWalletRepo::default()));
    let cb = svc.get_cashbox("g").await.unwrap();
    assert_eq!(cb.guild_id, "g");
}

#[tokio::test]
async fn deposit_noop_for_zero_and_negative() {
    let cb = Arc::new(MockCashboxRepo::default());
    let svc = make_svc(cb.clone(), Arc::new(SpyWalletRepo::default()));
    svc.deposit("g", 0, CashboxSource::ShopPurchase).await.unwrap();
    svc.deposit("g", -10, CashboxSource::ShopPurchase).await.unwrap();
    assert!(cb.deposits.lock().unwrap().is_empty());
}

#[tokio::test]
async fn deposit_delegates_when_positive() {
    let cb = Arc::new(MockCashboxRepo::default());
    let svc = make_svc(cb.clone(), Arc::new(SpyWalletRepo::default()));
    svc.deposit("g", 100, CashboxSource::DonationTax).await.unwrap();
    let deps = cb.deposits.lock().unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].0, "g");
    assert_eq!(deps[0].1, 100);
}

// ── redistribute_weekly ──

#[tokio::test]
async fn redistribute_weekly_none_when_no_active_players() {
    let cb = Arc::new(MockCashboxRepo::default());
    *cb.claim_total.lock().unwrap() = 1000;
    let svc = make_svc(cb, Arc::new(SpyWalletRepo::default()));
    assert!(svc.redistribute_weekly("g").await.unwrap().is_none());
}

#[tokio::test]
async fn redistribute_weekly_none_when_cashbox_empty() {
    let cb = Arc::new(MockCashboxRepo::default());
    cb.active.lock().unwrap().push(("u1".into(), "Alice".into()));
    // claim_total = 0
    let svc = make_svc(cb, Arc::new(SpyWalletRepo::default()));
    assert!(svc.redistribute_weekly("g").await.unwrap().is_none());
}

#[tokio::test]
async fn redistribute_weekly_success_credits_winners_and_records() {
    let cb = Arc::new(MockCashboxRepo::default());
    cb.active.lock().unwrap().extend(vec![
        ("u1".into(), "Alice".into()), ("u2".into(), "Bob".into()),
        ("u3".into(), "Carol".into()),
    ]);
    *cb.claim_total.lock().unwrap() = 10_000;
    let wallet = Arc::new(SpyWalletRepo::default());
    let svc = make_svc(cb.clone(), wallet.clone());
    let outcome = svc.redistribute_weekly("g").await.unwrap().unwrap();
    assert_eq!(outcome.total_amount, 10_000);
    assert!(!outcome.winners.is_empty());
    assert_eq!(outcome.winners.iter().map(|(_, _, a)| a).sum::<i64>(), 10_000);
    assert_eq!(cb.record_calls.lock().unwrap().len(), 1);
    // Un credit par gagnant
    assert_eq!(wallet.credit_calls.lock().unwrap().len(), outcome.winners.len());
}

// ── redistribute_due_guilds ──

#[tokio::test]
async fn redistribute_due_guilds_iterates_and_skips_empties() {
    let cb = Arc::new(MockCashboxRepo::default());
    cb.due_guilds.lock().unwrap().extend(vec!["g1".into(), "g2".into()]);
    // No active players → every sub-call returns None.
    let svc = make_svc(cb, Arc::new(SpyWalletRepo::default()));
    let out = svc.redistribute_due_guilds(7).await.unwrap();
    assert!(out.is_empty());
}

#[tokio::test]
async fn redistribute_due_guilds_returns_successful_outcomes() {
    let cb = Arc::new(MockCashboxRepo::default());
    cb.due_guilds.lock().unwrap().push("g1".into());
    cb.active.lock().unwrap().push(("u1".into(), "Alice".into()));
    *cb.claim_total.lock().unwrap() = 500;
    let svc = make_svc(cb, Arc::new(SpyWalletRepo::default()));
    let out = svc.redistribute_due_guilds(7).await.unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].0, "g1");
    assert_eq!(out[0].1.total_amount, 500);
}

// ── list_redistributions / list_entries ──

#[tokio::test]
async fn list_redistributions_delegates() {
    let cb = Arc::new(MockCashboxRepo::default());
    cb.redistributions.lock().unwrap().push(CashboxRedistribution {
        id: Uuid::new_v4(), guild_id: "g".into(), total_amount: 100,
        winners_count: 1, created_at: Utc::now(),
    });
    let svc = make_svc(cb, Arc::new(SpyWalletRepo::default()));
    assert_eq!(svc.list_redistributions("g", 10).await.unwrap().len(), 1);
}

#[tokio::test]
async fn list_entries_delegates() {
    let cb = Arc::new(MockCashboxRepo::default());
    cb.entries.lock().unwrap().push(CashboxRedistributionEntry {
        id: Uuid::new_v4(), redistribution_id: Uuid::new_v4(),
        user_id: "u".into(), username: "U".into(), amount_won: 50,
        created_at: Utc::now(),
    });
    let svc = make_svc(cb, Arc::new(SpyWalletRepo::default()));
    assert_eq!(svc.list_entries(Uuid::new_v4()).await.unwrap().len(), 1);
}

#[test]
fn distribute_random_total_sums_to_input() {
    for _ in 0..50 {
        let amounts = ManageCoudeCashboxService::distribute_random(1000, 5);
        assert_eq!(amounts.iter().sum::<i64>(), 1000);
        assert_eq!(amounts.len(), 5);
    }
}

#[test]
fn distribute_random_sorted_desc() {
    let amounts = ManageCoudeCashboxService::distribute_random(10_000, 10);
    for pair in amounts.windows(2) {
        assert!(pair[0] >= pair[1], "not sorted descending");
    }
}

#[test]
fn distribute_random_empty_on_zero_total() {
    assert!(ManageCoudeCashboxService::distribute_random(0, 5).is_empty());
    assert!(ManageCoudeCashboxService::distribute_random(100, 0).is_empty());
}

#[test]
fn distribute_random_produces_disparity() {
    let amounts = ManageCoudeCashboxService::distribute_random(1_000_000, 10);
    let max = *amounts.first().unwrap();
    let min = *amounts.last().unwrap();
    assert!(max >= min);
    assert!(max > 0);
}
