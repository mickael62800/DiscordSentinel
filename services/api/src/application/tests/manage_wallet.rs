use super::*;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::application::ManageWalletService;
use crate::domain::entities::{CoudeTauntsConfig, StreakKind, TauntEvent, Wallet, WalletTransaction};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_taunts::ManageCoudeTauntsUseCase;
use crate::ports::inbound::manage_wallet::ManageWalletUseCase;
use crate::ports::outbound::WalletRepository;

struct MockWalletRepo {
    balance: Mutex<i64>,
}
impl MockWalletRepo {
    fn new(initial: i64) -> Self {
        Self { balance: Mutex::new(initial) }
    }
    fn wallet(&self, coins: i64) -> Wallet {
        Wallet {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "u".into(),
            coins,
            total_earned: 0,
            total_spent: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
#[async_trait]
impl WalletRepository for MockWalletRepo {
    async fn get_or_create(&self, _g: &str, _u: &str, _n: &str, _s: i64) -> Result<Wallet, DomainError> {
        Ok(self.wallet(*self.balance.lock().unwrap()))
    }
    async fn get(&self, _g: &str, _u: &str) -> Result<Option<Wallet>, DomainError> {
        Ok(Some(self.wallet(*self.balance.lock().unwrap())))
    }
    async fn credit(&self, _g: &str, _u: &str, amount: i64, _s: &str, _d: &str) -> Result<Wallet, DomainError> {
        let mut b = self.balance.lock().unwrap();
        *b += amount;
        Ok(self.wallet(*b))
    }
    async fn debit(&self, _g: &str, _u: &str, amount: i64, _s: &str, _d: &str) -> Result<Wallet, DomainError> {
        let mut b = self.balance.lock().unwrap();
        if *b < amount {
            return Err(DomainError::ValidationError("insuffisant".into()));
        }
        *b -= amount;
        Ok(self.wallet(*b))
    }
    async fn transfer(&self, _g: &str, _f: &str, _t: &str, amount: i64, _s: &str, _d: &str) -> Result<(), DomainError> {
        let mut b = self.balance.lock().unwrap();
        if *b < amount {
            return Err(DomainError::ValidationError("insuffisant".into()));
        }
        *b -= amount;
        Ok(())
    }
    async fn pay_combat_atomic(&self, _: &str, _: &str, _: i64, _: &str, _: i64, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn leaderboard(&self, _g: &str, _l: i64) -> Result<Vec<Wallet>, DomainError> { Ok(vec![]) }
    async fn get_transactions(&self, _g: &str, _u: &str, _l: i64) -> Result<Vec<WalletTransaction>, DomainError> { Ok(vec![]) }
    async fn list_by_guild(&self, _g: &str) -> Result<Vec<Wallet>, DomainError> { Ok(vec![]) }
    async fn reset_wallet(&self, _g: &str, _u: &str, b: i64) -> Result<Wallet, DomainError> {
        *self.balance.lock().unwrap() = b;
        Ok(self.wallet(b))
    }
    async fn reset_all_wallets(&self, _g: &str, _b: i64) -> Result<u64, DomainError> { Ok(0) }
}

struct MockTaunts {
    bankruptcy_calls: Mutex<u32>,
    jackpot_calls: Mutex<u32>,
    last_jackpot_amount: Mutex<Option<i64>>,
}
impl MockTaunts {
    fn new() -> Self {
        Self {
            bankruptcy_calls: Mutex::new(0),
            jackpot_calls: Mutex::new(0),
            last_jackpot_amount: Mutex::new(None),
        }
    }
    fn fake_event(kind: StreakKind) -> TauntEvent {
        TauntEvent {
            channel_id: "c".into(),
            target_user_id: "u".into(),
            message: "taunt".into(),
            nickname_suffix: String::new(),
            streak_kind: kind.as_str(),
            streak_value: 1,
        }
    }
}
#[async_trait]
impl ManageCoudeTauntsUseCase for MockTaunts {
    async fn on_player_won(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_lost(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_drew(&self, _g: &str, _u: &str) -> Result<(), DomainError> { Ok(()) }
    async fn on_player_stolen_from(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_defended_steal(&self, _g: &str, _u: &str) -> Result<(), DomainError> { Ok(()) }
    async fn on_bj_natural(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_bj_hand_won(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_bj_hand_bust(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_bankruptcy(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> {
        *self.bankruptcy_calls.lock().unwrap() += 1;
        Ok(Some(Self::fake_event(StreakKind::EcoBankruptcy)))
    }
    async fn on_jackpot(&self, _g: &str, _u: &str, amount: i64) -> Result<Option<TauntEvent>, DomainError> {
        *self.jackpot_calls.lock().unwrap() += 1;
        *self.last_jackpot_amount.lock().unwrap() = Some(amount);
        if amount >= 10_000 {
            Ok(Some(Self::fake_event(StreakKind::EcoJackpot)))
        } else {
            Ok(None)
        }
    }
    async fn on_generous_donor(&self, _g: &str, _u: &str, _a: i64) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn get_config(&self, _g: &str) -> Result<CoudeTauntsConfig, DomainError> {
        Ok(CoudeTauntsConfig { guild_id: "g".into(), channel_id: None, enabled: false, rename_enabled: true, messages_enabled: true })
    }
    async fn set_channel(&self, _g: &str, _c: Option<&str>) -> Result<(), DomainError> { Ok(()) }
    async fn set_enabled(&self, _g: &str, _e: bool) -> Result<(), DomainError> { Ok(()) }
    async fn set_rename_enabled(&self, _g: &str, _e: bool) -> Result<(), DomainError> { Ok(()) }
    async fn set_messages_enabled(&self, _g: &str, _e: bool) -> Result<(), DomainError> { Ok(()) }
    async fn set_opt_out(&self, _g: &str, _u: &str, _o: bool) -> Result<(), DomainError> { Ok(()) }
    async fn is_opted_out(&self, _g: &str, _u: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn list_opt_outs(&self, _g: &str) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
}

#[tokio::test]
async fn credit_triggers_jackpot_when_amount_above_threshold() {
    let repo = Arc::new(MockWalletRepo::new(100));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo.clone(), taunts.clone());

    let m = svc.credit("g", "u", 15_000, "test", "d").await.unwrap();
    assert_eq!(m.new_balance, 15_100);
    assert_eq!(m.previous_balance, 100);
    assert_eq!(m.triggered_taunts.len(), 1);
    assert_eq!(*taunts.jackpot_calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn debit_full_balance_triggers_bankruptcy_taunt() {
    let repo = Arc::new(MockWalletRepo::new(500));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo.clone(), taunts.clone());

    let m = svc.debit("g", "u", 500, "test", "d").await.unwrap();
    assert_eq!(m.new_balance, 0);
    assert_eq!(m.previous_balance, 500);
    assert_eq!(m.triggered_taunts.len(), 1);
    assert_eq!(*taunts.bankruptcy_calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn debit_partial_does_not_trigger_bankruptcy() {
    let repo = Arc::new(MockWalletRepo::new(500));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo.clone(), taunts.clone());

    let m = svc.debit("g", "u", 100, "test", "d").await.unwrap();
    assert_eq!(m.triggered_taunts.len(), 0);
    assert_eq!(*taunts.bankruptcy_calls.lock().unwrap(), 0);
}

#[tokio::test]
async fn credit_rejects_non_positive_amount() {
    let repo = Arc::new(MockWalletRepo::new(100));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo, taunts);

    assert!(svc.credit("g", "u", 0, "t", "d").await.is_err());
    assert!(svc.credit("g", "u", -1, "t", "d").await.is_err());
}

#[tokio::test]
async fn debit_rejects_non_positive_amount() {
    let repo = Arc::new(MockWalletRepo::new(100));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo, taunts);

    assert!(svc.debit("g", "u", 0, "t", "d").await.is_err());
    assert!(svc.debit("g", "u", -5, "t", "d").await.is_err());
}

#[tokio::test]
async fn credit_below_jackpot_threshold_does_not_trigger() {
    let repo = Arc::new(MockWalletRepo::new(100));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo, taunts.clone());

    let m = svc.credit("g", "u", 500, "t", "d").await.unwrap();
    assert!(m.triggered_taunts.is_empty());
    // Mock is called regardless, but returns None under 10k.
    assert_eq!(*taunts.jackpot_calls.lock().unwrap(), 1);
    assert_eq!(*taunts.last_jackpot_amount.lock().unwrap(), Some(500));
}

#[tokio::test]
async fn debit_from_zero_does_not_trigger_bankruptcy() {
    // previous == 0, so strict transition >0 → 0 not met.
    let repo = Arc::new(MockWalletRepo::new(100));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo.clone(), taunts.clone());

    // Drain first (triggers one bankruptcy).
    let _ = svc.debit("g", "u", 100, "t", "d").await.unwrap();
    let before = *taunts.bankruptcy_calls.lock().unwrap();
    // Now balance is 0 ; debit of 0 is rejected, so do credit then debit partial.
    let _ = svc.credit("g", "u", 50, "t", "d").await.unwrap();
    let _ = svc.debit("g", "u", 20, "t", "d").await.unwrap();
    assert_eq!(*taunts.bankruptcy_calls.lock().unwrap(), before);
}

#[tokio::test]
async fn transfer_rejects_non_positive() {
    let repo = Arc::new(MockWalletRepo::new(500));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo, taunts);
    assert!(svc.transfer("g", "a", "b", 0, "t", "d").await.is_err());
    assert!(svc.transfer("g", "a", "b", -5, "t", "d").await.is_err());
}

#[tokio::test]
async fn transfer_rejects_self_transfer() {
    let repo = Arc::new(MockWalletRepo::new(500));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo, taunts);
    let err = svc.transfer("g", "alice", "alice", 100, "t", "d").await.unwrap_err();
    match err {
        DomainError::ValidationError(m) => assert!(m.contains("soi-meme")),
        o => panic!("expected ValidationError, got {:?}", o),
    }
}

#[tokio::test]
async fn transfer_full_balance_triggers_bankruptcy_and_jackpot() {
    // Sender drains to 0 (bankruptcy), receiver gets big amount (jackpot).
    let repo = Arc::new(MockWalletRepo::new(15_000));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo, taunts.clone());
    let events = svc.transfer("g", "alice", "bob", 15_000, "t", "d").await.unwrap();
    // Mock uses shared balance: sender before=15000, after=0. Receiver amount >= 10000 → jackpot.
    assert_eq!(*taunts.bankruptcy_calls.lock().unwrap(), 1);
    assert_eq!(*taunts.jackpot_calls.lock().unwrap(), 1);
    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn transfer_insufficient_balance_propagates_error() {
    let repo = Arc::new(MockWalletRepo::new(50));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo, taunts);
    assert!(svc.transfer("g", "a", "b", 500, "t", "d").await.is_err());
}

#[tokio::test]
async fn get_balance_reads_from_repo() {
    let repo = Arc::new(MockWalletRepo::new(1234));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo, taunts);
    assert_eq!(svc.get_balance("g", "u").await.unwrap(), 1234);
}

#[tokio::test]
async fn post_commit_taunts_emits_bankruptcy_and_jackpot() {
    use crate::ports::inbound::manage_wallet::TxWalletMutation;
    let repo = Arc::new(MockWalletRepo::new(0));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo, taunts.clone());

    let mutation = TxWalletMutation {
        new_balance: 0,
        previous_balance: 100,
        maybe_bankruptcy: true,
        maybe_jackpot_amount: Some(20_000),
    };
    let events = svc.post_commit_taunts("g", "u", &mutation).await;
    assert_eq!(events.len(), 2);
    assert_eq!(*taunts.bankruptcy_calls.lock().unwrap(), 1);
    assert_eq!(*taunts.jackpot_calls.lock().unwrap(), 1);
}

#[tokio::test]
async fn post_commit_taunts_skips_when_flags_unset() {
    use crate::ports::inbound::manage_wallet::TxWalletMutation;
    let repo = Arc::new(MockWalletRepo::new(0));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo, taunts.clone());

    let mutation = TxWalletMutation {
        new_balance: 0,
        previous_balance: 100,
        maybe_bankruptcy: false,
        maybe_jackpot_amount: None,
    };
    let events = svc.post_commit_taunts("g", "u", &mutation).await;
    assert!(events.is_empty());
    assert_eq!(*taunts.bankruptcy_calls.lock().unwrap(), 0);
    assert_eq!(*taunts.jackpot_calls.lock().unwrap(), 0);
}

#[tokio::test]
async fn post_commit_taunts_jackpot_below_threshold_emits_nothing() {
    use crate::ports::inbound::manage_wallet::TxWalletMutation;
    let repo = Arc::new(MockWalletRepo::new(0));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(repo, taunts.clone());

    let mutation = TxWalletMutation {
        new_balance: 500,
        previous_balance: 0,
        maybe_bankruptcy: false,
        maybe_jackpot_amount: Some(500),
    };
    let events = svc.post_commit_taunts("g", "u", &mutation).await;
    assert!(events.is_empty());
    assert_eq!(*taunts.jackpot_calls.lock().unwrap(), 1);
}
