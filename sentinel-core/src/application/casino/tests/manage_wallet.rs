use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::application::casino::manage_wallet_service::ManageWalletService;
use crate::domain::entities::casino::wallet::Wallet;
use crate::domain::entities::casino::wallet::WalletTransaction;
use crate::domain::entities::community::guild_member::GuildMember;
use crate::domain::entities::coude::taunt::StreakKind;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::entities::coude::taunt::TauntsConfig;
use crate::domain::entities::system::bot_config::BotDefinition;
use crate::domain::entities::system::bot_config::BotGuildConfig;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use crate::ports::outbound::community::member_repository::MemberRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::uow::DbTx;

struct MockWalletRepo {
    balance: Mutex<i64>,
    last_starting_coins: Mutex<Option<i64>>,
    last_reset_balance: Mutex<Option<i64>>,
    last_reset_all_balance: Mutex<Option<i64>>,
    reset_all_affected: u64,
    leaderboard_return: Mutex<Vec<Wallet>>,
    list_return: Mutex<Vec<Wallet>>,
    txs_return: Mutex<Vec<WalletTransaction>>,
}
impl MockWalletRepo {
    fn new(initial: i64) -> Self {
        Self {
            balance: Mutex::new(initial),
            last_starting_coins: Mutex::new(None),
            last_reset_balance: Mutex::new(None),
            last_reset_all_balance: Mutex::new(None),
            reset_all_affected: 0,
            leaderboard_return: Mutex::new(vec![]),
            list_return: Mutex::new(vec![]),
            txs_return: Mutex::new(vec![]),
        }
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
    async fn get_or_create(
        &self,
        _g: &str,
        _u: &str,
        _n: &str,
        s: i64,
    ) -> Result<Wallet, DomainError> {
        *self.last_starting_coins.lock().unwrap() = Some(s);
        Ok(self.wallet(*self.balance.lock().unwrap()))
    }
    async fn get(&self, _g: &str, _u: &str) -> Result<Option<Wallet>, DomainError> {
        Ok(Some(self.wallet(*self.balance.lock().unwrap())))
    }
    async fn credit(
        &self,
        _g: &str,
        _u: &str,
        amount: i64,
        _s: &str,
        _d: &str,
    ) -> Result<Wallet, DomainError> {
        let mut b = self.balance.lock().unwrap();
        *b += amount;
        Ok(self.wallet(*b))
    }
    async fn debit(
        &self,
        _g: &str,
        _u: &str,
        amount: i64,
        _s: &str,
        _d: &str,
    ) -> Result<Wallet, DomainError> {
        let mut b = self.balance.lock().unwrap();
        if *b < amount {
            return Err(DomainError::ValidationError("insuffisant".into()));
        }
        *b -= amount;
        Ok(self.wallet(*b))
    }
    async fn transfer(
        &self,
        _g: &str,
        _f: &str,
        _t: &str,
        amount: i64,
        _s: &str,
        _d: &str,
    ) -> Result<(), DomainError> {
        let mut b = self.balance.lock().unwrap();
        if *b < amount {
            return Err(DomainError::ValidationError("insuffisant".into()));
        }
        *b -= amount;
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
    async fn leaderboard(&self, _g: &str, _l: i64) -> Result<Vec<Wallet>, DomainError> {
        Ok(self.leaderboard_return.lock().unwrap().clone())
    }
    async fn get_transactions(
        &self,
        _g: &str,
        _u: &str,
        _l: i64,
    ) -> Result<Vec<WalletTransaction>, DomainError> {
        Ok(self.txs_return.lock().unwrap().clone())
    }
    async fn list_by_guild(&self, _g: &str) -> Result<Vec<Wallet>, DomainError> {
        Ok(self.list_return.lock().unwrap().clone())
    }
    async fn reset_wallet(&self, _g: &str, _u: &str, b: i64) -> Result<Wallet, DomainError> {
        *self.last_reset_balance.lock().unwrap() = Some(b);
        *self.balance.lock().unwrap() = b;
        Ok(self.wallet(b))
    }
    async fn reset_all_wallets(&self, _g: &str, b: i64) -> Result<u64, DomainError> {
        *self.last_reset_all_balance.lock().unwrap() = Some(b);
        Ok(self.reset_all_affected)
    }
    async fn credit_in_tx(
        &self,
        _tx: &mut dyn DbTx,
        _g: &str,
        _u: &str,
        _a: i64,
        _s: &str,
        _d: &str,
    ) -> Result<(i64, i64), DomainError> {
        unimplemented!()
    }
    async fn debit_in_tx(
        &self,
        _tx: &mut dyn DbTx,
        _g: &str,
        _u: &str,
        _a: i64,
        _s: &str,
        _d: &str,
    ) -> Result<(i64, i64), DomainError> {
        unimplemented!()
    }
}

struct MockMemberRepo;
#[async_trait]
impl MemberRepository for MockMemberRepo {
    async fn find_by_guild(&self, _g: &str) -> Result<Vec<GuildMember>, DomainError> {
        Ok(vec![])
    }
    async fn find_one(&self, _g: &str, _u: &str) -> Result<Option<GuildMember>, DomainError> {
        Ok(None)
    }
    async fn upsert(&self, _m: &GuildMember) -> Result<(), DomainError> {
        Ok(())
    }
    async fn upsert_many(&self, _m: &[GuildMember]) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn delete(&self, _g: &str, _u: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_last_seen(&self, _g: &str, _u: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn is_left(&self, _g: &str, _u: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn reset_member(
        &self,
        _g: &str,
        _u: &str,
    ) -> Result<Vec<(&'static str, u64)>, DomainError> {
        Ok(vec![])
    }
    async fn mark_left(&self, _g: &str, _u: &str) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn mark_rejoined(&self, _g: &str, _u: &str) -> Result<u64, DomainError> {
        Ok(0)
    }
}

struct MockBotConfigRepo;
#[async_trait]
impl BotConfigRepository for MockBotConfigRepo {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
        Ok(vec![])
    }
    async fn get_config(&self, _g: &str, _b: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn get_all_config(&self, _g: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
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
    async fn on_player_won(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_player_lost(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_player_drew(&self, _g: &str, _u: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn on_player_stolen_from(
        &self,
        _g: &str,
        _u: &str,
    ) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_player_defended_steal(&self, _g: &str, _u: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn on_bj_natural(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_bj_hand_won(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_bj_hand_bust(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn on_bankruptcy(&self, _g: &str, _u: &str) -> Result<Option<TauntEvent>, DomainError> {
        *self.bankruptcy_calls.lock().unwrap() += 1;
        Ok(Some(Self::fake_event(StreakKind::EcoBankruptcy)))
    }
    async fn on_jackpot(
        &self,
        _g: &str,
        _u: &str,
        amount: i64,
    ) -> Result<Option<TauntEvent>, DomainError> {
        *self.jackpot_calls.lock().unwrap() += 1;
        *self.last_jackpot_amount.lock().unwrap() = Some(amount);
        if amount >= 10_000 {
            Ok(Some(Self::fake_event(StreakKind::EcoJackpot)))
        } else {
            Ok(None)
        }
    }
    async fn on_generous_donor(
        &self,
        _g: &str,
        _u: &str,
        _a: i64,
    ) -> Result<Option<TauntEvent>, DomainError> {
        Ok(None)
    }
    async fn get_config(&self, _g: &str) -> Result<TauntsConfig, DomainError> {
        Ok(TauntsConfig {
            guild_id: "g".into(),
            channel_id: None,
            enabled: false,
            rename_enabled: true,
            messages_enabled: true,
        })
    }
    async fn set_channel(&self, _g: &str, _c: Option<&str>) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_enabled(&self, _g: &str, _e: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_rename_enabled(&self, _g: &str, _e: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_messages_enabled(&self, _g: &str, _e: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_opt_out(&self, _g: &str, _u: &str, _o: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn is_opted_out(&self, _g: &str, _u: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn list_opt_outs(&self, _g: &str) -> Result<Vec<String>, DomainError> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn credit_triggers_jackpot_when_amount_above_threshold() {
    let repo = Arc::new(MockWalletRepo::new(100));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo.clone(),
        taunts.clone(),
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

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
    let svc = ManageWalletService::new(
        repo.clone(),
        taunts.clone(),
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

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
    let svc = ManageWalletService::new(
        repo.clone(),
        taunts.clone(),
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

    let m = svc.debit("g", "u", 100, "test", "d").await.unwrap();
    assert_eq!(m.triggered_taunts.len(), 0);
    assert_eq!(*taunts.bankruptcy_calls.lock().unwrap(), 0);
}

#[tokio::test]
async fn credit_rejects_non_positive_amount() {
    let repo = Arc::new(MockWalletRepo::new(100));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo,
        taunts,
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

    assert!(svc.credit("g", "u", 0, "t", "d").await.is_err());
    assert!(svc.credit("g", "u", -1, "t", "d").await.is_err());
}

#[tokio::test]
async fn debit_rejects_non_positive_amount() {
    let repo = Arc::new(MockWalletRepo::new(100));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo,
        taunts,
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

    assert!(svc.debit("g", "u", 0, "t", "d").await.is_err());
    assert!(svc.debit("g", "u", -5, "t", "d").await.is_err());
}

#[tokio::test]
async fn credit_below_jackpot_threshold_does_not_trigger() {
    let repo = Arc::new(MockWalletRepo::new(100));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo,
        taunts.clone(),
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

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
    let svc = ManageWalletService::new(
        repo.clone(),
        taunts.clone(),
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

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
    let svc = ManageWalletService::new(
        repo,
        taunts,
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );
    assert!(svc.transfer("g", "a", "b", 0, "t", "d").await.is_err());
    assert!(svc.transfer("g", "a", "b", -5, "t", "d").await.is_err());
}

#[tokio::test]
async fn transfer_rejects_self_transfer() {
    let repo = Arc::new(MockWalletRepo::new(500));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo,
        taunts,
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );
    let err = svc
        .transfer("g", "alice", "alice", 100, "t", "d")
        .await
        .unwrap_err();
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
    let svc = ManageWalletService::new(
        repo,
        taunts.clone(),
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );
    let events = svc
        .transfer("g", "alice", "bob", 15_000, "t", "d")
        .await
        .unwrap();
    // Mock uses shared balance: sender before=15000, after=0. Receiver amount >= 10000 → jackpot.
    assert_eq!(*taunts.bankruptcy_calls.lock().unwrap(), 1);
    assert_eq!(*taunts.jackpot_calls.lock().unwrap(), 1);
    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn transfer_insufficient_balance_propagates_error() {
    let repo = Arc::new(MockWalletRepo::new(50));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo,
        taunts,
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );
    assert!(svc.transfer("g", "a", "b", 500, "t", "d").await.is_err());
}

#[tokio::test]
async fn get_balance_reads_from_repo() {
    let repo = Arc::new(MockWalletRepo::new(1234));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo,
        taunts,
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );
    assert_eq!(svc.get_balance("g", "u").await.unwrap(), 1234);
}

#[tokio::test]
async fn post_commit_taunts_emits_bankruptcy_and_jackpot() {
    use crate::ports::inbound::casino::manage_wallet::TxWalletMutation;
    let repo = Arc::new(MockWalletRepo::new(0));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo,
        taunts.clone(),
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

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
    use crate::ports::inbound::casino::manage_wallet::TxWalletMutation;
    let repo = Arc::new(MockWalletRepo::new(0));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo,
        taunts.clone(),
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

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

// ── Lectures + admin (handler delegation) ─────────────────────────────

#[tokio::test]
async fn get_or_create_uses_default_starting_coins_when_env_absent() {
    std::env::remove_var("WALLET_STARTING_COINS");
    let repo = Arc::new(MockWalletRepo::new(0));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo.clone(),
        taunts,
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

    let _ = svc.get_or_create("g", "u").await.unwrap();
    assert_eq!(*repo.last_starting_coins.lock().unwrap(), Some(100));
}

#[tokio::test]
async fn list_by_guild_delegates_to_repo() {
    let repo = Arc::new(MockWalletRepo::new(0));
    let w = repo.wallet(42);
    *repo.list_return.lock().unwrap() = vec![w];
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo.clone(),
        taunts,
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

    let out = svc.list_by_guild("g").await.unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].coins, 42);
}

#[tokio::test]
async fn leaderboard_delegates_to_repo() {
    let repo = Arc::new(MockWalletRepo::new(0));
    *repo.leaderboard_return.lock().unwrap() = vec![repo.wallet(10), repo.wallet(5)];
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo.clone(),
        taunts,
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

    let out = svc.leaderboard("g", 20).await.unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].coins, 10);
}

#[tokio::test]
async fn get_transactions_delegates_to_repo() {
    let repo = Arc::new(MockWalletRepo::new(0));
    *repo.txs_return.lock().unwrap() = vec![WalletTransaction {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        user_id: "u".into(),
        amount: 50,
        balance_after: 150,
        source: "test".into(),
        description: "d".into(),
        created_at: Utc::now(),
    }];
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo.clone(),
        taunts,
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

    let out = svc.get_transactions("g", "u", 10).await.unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].amount, 50);
}

#[tokio::test]
async fn reset_wallet_applies_resolve_reset_balance() {
    let repo = Arc::new(MockWalletRepo::new(999));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo.clone(),
        taunts,
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

    // None → defaut 100
    let (_, nb) = svc.reset_wallet("g", "u", None).await.unwrap();
    assert_eq!(nb, 100);
    assert_eq!(*repo.last_reset_balance.lock().unwrap(), Some(100));

    // Negatif → clampe a 0
    let (_, nb) = svc.reset_wallet("g", "u", Some(-500)).await.unwrap();
    assert_eq!(nb, 0);
    assert_eq!(*repo.last_reset_balance.lock().unwrap(), Some(0));

    // Valeur positive → passee telle quelle
    let (w, nb) = svc.reset_wallet("g", "u", Some(777)).await.unwrap();
    assert_eq!(nb, 777);
    assert_eq!(w.coins, 777);
    assert_eq!(*repo.last_reset_balance.lock().unwrap(), Some(777));
}

#[tokio::test]
async fn reset_all_wallets_applies_resolve_reset_balance_and_returns_affected() {
    let mut repo = MockWalletRepo::new(0);
    repo.reset_all_affected = 42;
    let repo = Arc::new(repo);
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo.clone(),
        taunts,
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

    let (affected, nb) = svc.reset_all_wallets("g", Some(-10)).await.unwrap();
    assert_eq!(affected, 42);
    assert_eq!(nb, 0);
    assert_eq!(*repo.last_reset_all_balance.lock().unwrap(), Some(0));

    let (_, nb) = svc.reset_all_wallets("g", None).await.unwrap();
    assert_eq!(nb, 100);
}

#[tokio::test]
async fn post_commit_taunts_jackpot_below_threshold_emits_nothing() {
    use crate::ports::inbound::casino::manage_wallet::TxWalletMutation;
    let repo = Arc::new(MockWalletRepo::new(0));
    let taunts = Arc::new(MockTaunts::new());
    let svc = ManageWalletService::new(
        repo,
        taunts.clone(),
        Arc::new(MockMemberRepo),
        Arc::new(MockBotConfigRepo),
    );

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
