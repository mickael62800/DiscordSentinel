use std::sync::Arc;
use std::sync::Mutex;
use async_trait::async_trait;
use sqlx::Postgres;
use sqlx::Transaction;
use crate::application::coude::manage_economy_service::ManageCoudeEconomyService;
use crate::domain::entities::coude::taunt::TauntsConfig;
use crate::domain::entities::coude::taunt::StreakKind;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_economy::ManageCoudeEconomyUseCase;
use crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::casino::manage_wallet::TxWalletMutation;
use crate::ports::inbound::casino::manage_wallet::WalletMutation;
use crate::ports::outbound::coude::economy_repository::EconomyRepository;

struct MockEconomyRepo {
    coins: Mutex<std::collections::HashMap<String, i64>>,
    stats_calls: Mutex<Vec<(String, String, String, i64)>>,
    fail_stats_calls: Mutex<Vec<(String, String, i64)>>,
    casino_win_stats: Mutex<Vec<(String, String, i64)>>,
    casino_loss_stats: Mutex<Vec<(String, String, i64)>>,
    casino_faillite_stats: Mutex<Vec<(String, String, i64)>>,
}
impl MockEconomyRepo {
    fn new() -> Self {
        Self {
            coins: Mutex::new(std::collections::HashMap::new()),
            stats_calls: Mutex::new(Vec::new()),
            fail_stats_calls: Mutex::new(Vec::new()),
            casino_win_stats: Mutex::new(Vec::new()),
            casino_loss_stats: Mutex::new(Vec::new()),
            casino_faillite_stats: Mutex::new(Vec::new()),
        }
    }
    fn set_coins(&self, guild_id: &str, user_id: &str, coins: i64) {
        self.coins.lock().unwrap().insert(format!("{}:{}", guild_id, user_id), coins);
    }
}
#[async_trait]
impl EconomyRepository for MockEconomyRepo {
    async fn record_steal_stats(&self, g: &str, thief: &str, victim: &str, amount: i64) -> Result<(), DomainError> {
        self.stats_calls.lock().unwrap().push((g.into(), thief.into(), victim.into(), amount));
        Ok(())
    }
    async fn record_steal_fail_stats(&self, g: &str, thief: &str, amount: i64) -> Result<(), DomainError> {
        self.fail_stats_calls.lock().unwrap().push((g.into(), thief.into(), amount));
        Ok(())
    }
    async fn get_coins(&self, g: &str, u: &str) -> Result<i64, DomainError> {
        self.coins.lock().unwrap().get(&format!("{}:{}", g, u)).copied()
            .ok_or_else(|| DomainError::NotFound("Wallet introuvable".into()))
    }
    async fn record_casino_win_stats(&self, g: &str, u: &str, gain: i64) -> Result<(), DomainError> {
        self.casino_win_stats.lock().unwrap().push((g.into(), u.into(), gain));
        Ok(())
    }
    async fn record_casino_loss_stats(&self, g: &str, u: &str, lost: i64) -> Result<(), DomainError> {
        self.casino_loss_stats.lock().unwrap().push((g.into(), u.into(), lost));
        Ok(())
    }
    async fn record_casino_faillite_stats(&self, g: &str, u: &str, cleared: i64) -> Result<i64, DomainError> {
        self.casino_faillite_stats.lock().unwrap().push((g.into(), u.into(), cleared));
        Ok(cleared)
    }
    async fn count_casino_today(&self, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
    async fn sum_casino_gains_today(&self, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
    async fn count_steal_today(&self, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
}

fn fake_taunt(kind: StreakKind, user: &str) -> TauntEvent {
    TauntEvent {
        channel_id: "chan".into(),
        target_user_id: user.into(),
        message: format!("taunt {}", kind.as_str()),
        nickname_suffix: String::new(),
        streak_kind: kind.as_str(),
        streak_value: 1,
    }
}

struct MockWalletUc {
    returned: Vec<TauntEvent>,
    calls: Mutex<Vec<(String, String, String, i64, String)>>,
    debit_calls: Mutex<Vec<(String, String, i64, String)>>,
    debit_returned: Vec<TauntEvent>,
    credit_calls: Mutex<Vec<(String, String, i64, String)>>,
    credit_returned: Vec<TauntEvent>,
    balances: Mutex<std::collections::HashMap<String, i64>>,
    should_fail: bool,
}
impl MockWalletUc {
    fn set_balance(&self, guild_id: &str, user_id: &str, coins: i64) {
        self.balances.lock().unwrap().insert(format!("{}:{}", guild_id, user_id), coins);
    }
}
#[async_trait]
impl ManageWalletUseCase for MockWalletUc {
    async fn credit(&self, guild_id: &str, user: &str, amount: i64, source: &str, _desc: &str) -> Result<WalletMutation, DomainError> {
        if self.should_fail {
            return Err(DomainError::ValidationError("wallet fail".into()));
        }
        self.credit_calls.lock().unwrap().push((guild_id.into(), user.into(), amount, source.into()));
        let key = format!("{}:{}", guild_id, user);
        let mut map = self.balances.lock().unwrap();
        let prev = *map.get(&key).unwrap_or(&0);
        let new_balance = prev + amount;
        map.insert(key, new_balance);
        Ok(WalletMutation { new_balance, previous_balance: prev, triggered_taunts: self.credit_returned.clone() })
    }
    async fn debit(&self, guild_id: &str, user: &str, amount: i64, source: &str, _desc: &str) -> Result<WalletMutation, DomainError> {
        if self.should_fail {
            return Err(DomainError::ValidationError("Solde insuffisant".into()));
        }
        self.debit_calls.lock().unwrap().push((guild_id.into(), user.into(), amount, source.into()));
        Ok(WalletMutation { new_balance: 0, previous_balance: amount, triggered_taunts: self.debit_returned.clone() })
    }
    async fn transfer(&self, guild_id: &str, from: &str, to: &str, amount: i64, source: &str, _desc: &str) -> Result<Vec<TauntEvent>, DomainError> {
        if self.should_fail {
            return Err(DomainError::ValidationError("Solde insuffisant".into()));
        }
        self.calls.lock().unwrap().push((guild_id.into(), from.into(), to.into(), amount, source.into()));
        Ok(self.returned.clone())
    }
    async fn get_balance(&self, guild_id: &str, user_id: &str) -> Result<i64, DomainError> {
        Ok(*self.balances.lock().unwrap().get(&format!("{}:{}", guild_id, user_id)).unwrap_or(&0))
    }
    async fn credit_tx(&self, _: &mut Transaction<'_, Postgres>, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<TxWalletMutation, DomainError> { unimplemented!() }
    async fn debit_tx(&self, _: &mut Transaction<'_, Postgres>, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<TxWalletMutation, DomainError> { unimplemented!() }
    async fn post_commit_taunts(&self, _: &str, _: &str, _: &TxWalletMutation) -> Vec<TauntEvent> { vec![] }
}

struct MockTauntsUc {
    donor_threshold: i64,
    donor_calls: Mutex<Vec<(String, String, i64)>>,
}
#[async_trait]
impl ManageCoudeTauntsUseCase for MockTauntsUc {
    async fn on_player_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_lost(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_drew(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn on_player_stolen_from(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_player_defended_steal(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn on_bj_natural(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_bj_hand_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_bj_hand_bust(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_bankruptcy(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_jackpot(&self, _: &str, _: &str, _: i64) -> Result<Option<TauntEvent>, DomainError> { Ok(None) }
    async fn on_generous_donor(&self, guild_id: &str, user_id: &str, amount: i64) -> Result<Option<TauntEvent>, DomainError> {
        self.donor_calls.lock().unwrap().push((guild_id.into(), user_id.into(), amount));
        if amount >= self.donor_threshold {
            Ok(Some(fake_taunt(StreakKind::EcoGenerousDonor, user_id)))
        } else {
            Ok(None)
        }
    }
    async fn get_config(&self, _: &str) -> Result<TauntsConfig, DomainError> {
        Ok(TauntsConfig { guild_id: "g".into(), channel_id: None, enabled: false, rename_enabled: true, messages_enabled: true })
    }
    async fn set_channel(&self, _: &str, _: Option<&str>) -> Result<(), DomainError> { Ok(()) }
    async fn set_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn set_rename_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn set_messages_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn set_opt_out(&self, _: &str, _: &str, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn is_opted_out(&self, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn list_opt_outs(&self, _: &str) -> Result<Vec<String>, DomainError> { Ok(vec![]) }
}

fn build_service(
    wallet_taunts: Vec<TauntEvent>,
    wallet_fail: bool,
    donor_threshold: i64,
) -> (ManageCoudeEconomyService, Arc<MockEconomyRepo>, Arc<MockWalletUc>, Arc<MockTauntsUc>) {
    build_service_with_debit_taunts(wallet_taunts, vec![], wallet_fail, donor_threshold)
}

fn build_service_with_debit_taunts(
    wallet_taunts: Vec<TauntEvent>,
    debit_taunts: Vec<TauntEvent>,
    wallet_fail: bool,
    donor_threshold: i64,
) -> (ManageCoudeEconomyService, Arc<MockEconomyRepo>, Arc<MockWalletUc>, Arc<MockTauntsUc>) {
    let repo = Arc::new(MockEconomyRepo::new());
    let wallet = Arc::new(MockWalletUc {
        returned: wallet_taunts,
        calls: Mutex::new(Vec::new()),
        debit_calls: Mutex::new(Vec::new()),
        debit_returned: debit_taunts,
        credit_calls: Mutex::new(Vec::new()),
        credit_returned: Vec::new(),
        balances: Mutex::new(std::collections::HashMap::new()),
        should_fail: wallet_fail,
    });
    let taunts = Arc::new(MockTauntsUc { donor_threshold, donor_calls: Mutex::new(Vec::new()) });
    let svc = ManageCoudeEconomyService::new(repo.clone(), wallet.clone(), taunts.clone());
    (svc, repo, wallet, taunts)
}

#[tokio::test]
async fn transfer_delegates_to_wallet_uc_and_concats_donor_taunt() {
    let wallet_taunts = vec![fake_taunt(StreakKind::EcoBankruptcy, "alice")];
    let (svc, _repo, wallet, taunts) = build_service(wallet_taunts, false, 1_000);
    let out = svc.transfer("g1", "alice", "bob", 5_000).await.unwrap();
    let calls = wallet.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "g1");
    assert_eq!(calls[0].1, "alice");
    assert_eq!(calls[0].2, "bob");
    assert_eq!(calls[0].3, 5_000);
    assert_eq!(calls[0].4, "coude_donner");
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].streak_kind, StreakKind::EcoBankruptcy.as_str());
    assert_eq!(out[1].streak_kind, StreakKind::EcoGenerousDonor.as_str());
    let donor_calls = taunts.donor_calls.lock().unwrap().clone();
    assert_eq!(donor_calls.len(), 1);
    assert_eq!(donor_calls[0].2, 5_000);
}

#[tokio::test]
async fn transfer_below_donor_threshold_does_not_trigger_donor_taunt() {
    let (svc, _repo, _wallet, _taunts) = build_service(vec![], false, 10_000);
    let out = svc.transfer("g1", "alice", "bob", 500).await.unwrap();
    assert!(out.is_empty());
}

#[tokio::test]
async fn transfer_rejects_self_transfer_before_calling_wallet() {
    let (svc, _repo, wallet, _) = build_service(vec![], false, 1);
    let err = svc.transfer("g1", "alice", "alice", 100).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
    assert!(wallet.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn transfer_rejects_non_positive_amount() {
    let (svc, _repo, wallet, _) = build_service(vec![], false, 1);
    assert!(svc.transfer("g1", "a", "b", 0).await.is_err());
    assert!(svc.transfer("g1", "a", "b", -10).await.is_err());
    assert!(wallet.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn transfer_propagates_wallet_error() {
    let (svc, _repo, _, _) = build_service(vec![], true, 1);
    let err = svc.transfer("g1", "alice", "bob", 100).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn steal_success_delegates_to_wallet_transfer() {
    let wallet_taunts = vec![fake_taunt(StreakKind::EcoJackpot, "thief")];
    let (svc, repo, wallet, _taunts) = build_service(wallet_taunts, false, 9_999_999);
    repo.set_coins("g1", "victim", 5000);
    let outcome = svc.steal("g1", "thief", "victim", 1000).await.unwrap();
    assert_eq!(outcome.stolen, 1000);
    assert_eq!(outcome.taunt_events.len(), 1);
    assert_eq!(outcome.taunt_events[0].streak_kind, StreakKind::EcoJackpot.as_str());
    let calls = wallet.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "g1");
    assert_eq!(calls[0].1, "victim");
    assert_eq!(calls[0].2, "thief");
    assert_eq!(calls[0].3, 1000);
    assert_eq!(calls[0].4, "coude_steal_success");
    let stats = repo.stats_calls.lock().unwrap().clone();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].1, "thief");
    assert_eq!(stats[0].2, "victim");
    assert_eq!(stats[0].3, 1000);
}

#[tokio::test]
async fn steal_clamps_to_victim_balance() {
    let (svc, repo, wallet, _) = build_service(vec![], false, 9_999_999);
    repo.set_coins("g1", "victim", 800);
    let outcome = svc.steal("g1", "thief", "victim", 5000).await.unwrap();
    assert_eq!(outcome.stolen, 800);
    let calls = wallet.calls.lock().unwrap().clone();
    assert_eq!(calls[0].3, 800);
}

#[tokio::test]
async fn steal_rejects_when_victim_has_nothing() {
    let (svc, repo, wallet, _) = build_service(vec![], false, 1);
    repo.set_coins("g1", "victim", 0);
    let err = svc.steal("g1", "thief", "victim", 100).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
    assert!(wallet.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn steal_rejects_self_steal() {
    let (svc, _repo, wallet, _) = build_service(vec![], false, 1);
    let err = svc.steal("g1", "alice", "alice", 100).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
    assert!(wallet.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn steal_rejects_non_positive_amount() {
    let (svc, repo, wallet, _) = build_service(vec![], false, 1);
    repo.set_coins("g1", "victim", 1000);
    assert!(svc.steal("g1", "thief", "victim", 0).await.is_err());
    assert!(svc.steal("g1", "thief", "victim", -10).await.is_err());
    assert!(wallet.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn steal_fail_penalty_delegates_to_wallet_debit() {
    let (svc, repo, wallet, _) = build_service_with_debit_taunts(vec![], vec![], false, 1);
    repo.set_coins("g1", "thief", 2000);
    let (lost, taunts) = svc.steal_fail_penalty("g1", "thief", 500).await.unwrap();
    assert_eq!(lost, 500);
    assert!(taunts.is_empty());
    let debit_calls = wallet.debit_calls.lock().unwrap().clone();
    assert_eq!(debit_calls.len(), 1);
    assert_eq!(debit_calls[0].0, "g1");
    assert_eq!(debit_calls[0].1, "thief");
    assert_eq!(debit_calls[0].2, 500);
    assert_eq!(debit_calls[0].3, "coude_steal_fail_penalty");
    let stats = repo.fail_stats_calls.lock().unwrap().clone();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].1, "thief");
    assert_eq!(stats[0].2, 500);
}

#[tokio::test]
async fn steal_fail_penalty_clamps_to_thief_balance() {
    let (svc, repo, wallet, _) = build_service(vec![], false, 1);
    repo.set_coins("g1", "thief", 300);
    let (lost, _taunts) = svc.steal_fail_penalty("g1", "thief", 1000).await.unwrap();
    assert_eq!(lost, 300);
    let debit_calls = wallet.debit_calls.lock().unwrap().clone();
    assert_eq!(debit_calls[0].2, 300);
}

#[tokio::test]
async fn steal_fail_penalty_noop_when_thief_has_nothing() {
    let (svc, repo, wallet, _) = build_service(vec![], false, 1);
    repo.set_coins("g1", "thief", 0);
    let (lost, taunts) = svc.steal_fail_penalty("g1", "thief", 500).await.unwrap();
    assert_eq!(lost, 0);
    assert!(taunts.is_empty());
    assert!(wallet.debit_calls.lock().unwrap().is_empty());
    assert!(repo.fail_stats_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn steal_fail_penalty_propagates_bankruptcy_taunt() {
    let (svc, repo, _wallet, _) = build_service_with_debit_taunts(
        vec![],
        vec![fake_taunt(StreakKind::EcoBankruptcy, "thief")],
        false,
        1,
    );
    repo.set_coins("g1", "thief", 1000);
    let (lost, taunts) = svc.steal_fail_penalty("g1", "thief", 1000).await.unwrap();
    assert_eq!(lost, 1000);
    assert_eq!(taunts.len(), 1);
    assert_eq!(taunts[0].streak_kind, StreakKind::EcoBankruptcy.as_str());
}

#[tokio::test]
async fn casino_win_delegates_to_wallet_credit() {
    let (svc, repo, wallet, _) = build_service(vec![], false, 1);
    svc.record_casino_win("g1", "alice", 1500).await.unwrap();
    let credit_calls = wallet.credit_calls.lock().unwrap().clone();
    assert_eq!(credit_calls.len(), 1);
    assert_eq!(credit_calls[0].0, "g1");
    assert_eq!(credit_calls[0].1, "alice");
    assert_eq!(credit_calls[0].2, 1500);
    assert_eq!(credit_calls[0].3, "coude_casino_win");
    let stats = repo.casino_win_stats.lock().unwrap().clone();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0], ("g1".into(), "alice".into(), 1500));
}

#[tokio::test]
async fn casino_loss_delegates_to_wallet_debit_clamped_to_balance() {
    let (svc, repo, wallet, _) = build_service(vec![], false, 1);
    wallet.set_balance("g1", "alice", 800);
    svc.record_casino_loss("g1", "alice", 1500).await.unwrap();
    let debit_calls = wallet.debit_calls.lock().unwrap().clone();
    assert_eq!(debit_calls.len(), 1);
    assert_eq!(debit_calls[0].2, 800);
    assert_eq!(debit_calls[0].3, "coude_casino_loss");
    let stats = repo.casino_loss_stats.lock().unwrap().clone();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0], ("g1".into(), "alice".into(), 1500));
}

#[tokio::test]
async fn casino_faillite_debits_full_balance_and_records_stats() {
    let (svc, repo, wallet, _) = build_service(vec![], false, 1);
    wallet.set_balance("g1", "alice", 2000);
    let total_lost = svc.record_casino_faillite("g1", "alice").await.unwrap();
    assert_eq!(total_lost, 2000);
    let debit_calls = wallet.debit_calls.lock().unwrap().clone();
    assert_eq!(debit_calls.len(), 1);
    assert_eq!(debit_calls[0].2, 2000);
    assert_eq!(debit_calls[0].3, "coude_casino_faillite");
    let stats = repo.casino_faillite_stats.lock().unwrap().clone();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0], ("g1".into(), "alice".into(), 2000));
}

#[tokio::test]
async fn casino_faillite_on_empty_wallet_only_records_stats() {
    let (svc, repo, wallet, _) = build_service(vec![], false, 1);
    wallet.set_balance("g1", "alice", 0);
    let total_lost = svc.record_casino_faillite("g1", "alice").await.unwrap();
    assert_eq!(total_lost, 0);
    assert!(wallet.debit_calls.lock().unwrap().is_empty());
    let stats = repo.casino_faillite_stats.lock().unwrap().clone();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].2, 0);
}

// ── Validation et chemins zero ──

#[tokio::test]
async fn casino_win_rejects_negative_gain() {
    let (svc, _, _, _) = build_service(vec![], false, 1);
    let err = svc.record_casino_win("g1", "alice", -100).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn casino_win_zero_gain_skips_credit_but_records_stats() {
    let (svc, repo, wallet, _) = build_service(vec![], false, 1);
    svc.record_casino_win("g1", "alice", 0).await.unwrap();
    assert!(wallet.credit_calls.lock().unwrap().is_empty());
    assert_eq!(repo.casino_win_stats.lock().unwrap().len(), 1);
    assert_eq!(repo.casino_win_stats.lock().unwrap()[0].2, 0);
}

#[tokio::test]
async fn casino_loss_rejects_negative() {
    let (svc, _, _, _) = build_service(vec![], false, 1);
    let err = svc.record_casino_loss("g1", "alice", -50).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn casino_loss_zero_skips_debit_but_records_stats() {
    let (svc, repo, wallet, _) = build_service(vec![], false, 1);
    svc.record_casino_loss("g1", "alice", 0).await.unwrap();
    assert!(wallet.debit_calls.lock().unwrap().is_empty());
    assert_eq!(repo.casino_loss_stats.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn casino_loss_with_zero_balance_skips_debit() {
    let (svc, repo, wallet, _) = build_service(vec![], false, 1);
    wallet.set_balance("g1", "alice", 0);
    svc.record_casino_loss("g1", "alice", 500).await.unwrap();
    assert!(wallet.debit_calls.lock().unwrap().is_empty());
    // Stats quand meme enregistrees avec le montant nominal
    assert_eq!(repo.casino_loss_stats.lock().unwrap()[0].2, 500);
}

#[tokio::test]
async fn steal_fail_penalty_zero_amount_returns_empty() {
    let (svc, _, _, _) = build_service(vec![], false, 1);
    let (lost, taunts) = svc.steal_fail_penalty("g1", "thief", 0).await.unwrap();
    assert_eq!(lost, 0);
    assert!(taunts.is_empty());
}

#[tokio::test]
async fn steal_fail_penalty_negative_amount_returns_empty() {
    let (svc, _, _, _) = build_service(vec![], false, 1);
    let (lost, _) = svc.steal_fail_penalty("g1", "thief", -100).await.unwrap();
    assert_eq!(lost, 0);
}
