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
        Ok(CoudeTauntsConfig { guild_id: "g".into(), channel_id: None, enabled: false })
    }
    async fn set_channel(&self, _g: &str, _c: Option<&str>) -> Result<(), DomainError> { Ok(()) }
    async fn set_enabled(&self, _g: &str, _e: bool) -> Result<(), DomainError> { Ok(()) }
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
