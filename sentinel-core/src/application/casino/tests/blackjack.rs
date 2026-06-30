use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::application::casino::blackjack_service::BlackjackService;
use crate::domain::entities::casino::blackjack::BlackjackGame;
use crate::domain::entities::casino::wallet::Wallet;
use crate::domain::entities::casino::wallet::WalletTransaction;
use crate::domain::entities::coude::taunt::StreakKind;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::casino::manage_wallet::TxWalletMutation;
use crate::ports::inbound::casino::manage_wallet::WalletMutation;
use crate::ports::outbound::casino::blackjack_repository::BlackjackRepository;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
fn fake_taunt(kind: StreakKind) -> TauntEvent {
    TauntEvent {
        channel_id: "chan".into(),
        target_user_id: "u".into(),
        message: "boom".into(),
        nickname_suffix: String::new(),
        streak_kind: kind.as_str(),
        streak_value: 1,
    }
}

struct FakeBlackjackRepo {
    created: Mutex<Option<BlackjackGame>>,
}
#[async_trait]
impl BlackjackRepository for FakeBlackjackRepo {
    async fn create(&self, game: &BlackjackGame) -> Result<(), DomainError> {
        *self.created.lock().unwrap() = Some(game.clone());
        Ok(())
    }
    async fn get_active(&self, _: &str, _: &str) -> Result<Option<BlackjackGame>, DomainError> {
        Ok(None)
    }
    async fn update(&self, _: &BlackjackGame) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_by_id(&self, _: Uuid) -> Result<Option<BlackjackGame>, DomainError> {
        Ok(None)
    }
    async fn list_by_guild(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> Result<Vec<BlackjackGame>, DomainError> {
        Ok(vec![])
    }
    async fn cancel_game(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
}

struct FakeWalletRepo;
#[async_trait]
impl WalletRepository for FakeWalletRepo {
    async fn get_or_create(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Wallet, DomainError> {
        Ok(Wallet {
            id: Uuid::nil(),
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "x".into(),
            coins: 500,
            total_earned: 0,
            total_spent: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
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
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<Wallet, DomainError> {
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
    ) -> Result<(), DomainError> {
        unimplemented!()
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
    async fn leaderboard(&self, _: &str, _: i64) -> Result<Vec<Wallet>, DomainError> {
        unimplemented!()
    }
    async fn get_transactions(
        &self,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Vec<WalletTransaction>, DomainError> {
        unimplemented!()
    }
    async fn list_by_guild(&self, _: &str) -> Result<Vec<Wallet>, DomainError> {
        unimplemented!()
    }
    async fn reset_wallet(&self, _: &str, _: &str, _: i64) -> Result<Wallet, DomainError> {
        unimplemented!()
    }
    async fn reset_all_wallets(&self, _: &str, _: i64) -> Result<u64, DomainError> {
        unimplemented!()
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

struct MockWalletUc {
    debit_taunts: Vec<TauntEvent>,
    credit_taunts: Vec<TauntEvent>,
    debit_should_fail: bool,
    calls: Mutex<Vec<String>>,
}
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
        self.calls.lock().unwrap().push("credit".into());
        Ok(WalletMutation {
            new_balance: 100,
            previous_balance: 0,
            triggered_taunts: self.credit_taunts.clone(),
        })
    }
    async fn debit(
        &self,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
        _: &str,
    ) -> Result<WalletMutation, DomainError> {
        self.calls.lock().unwrap().push("debit".into());
        if self.debit_should_fail {
            return Err(DomainError::ValidationError("Solde insuffisant".into()));
        }
        Ok(WalletMutation {
            new_balance: 0,
            previous_balance: 100,
            triggered_taunts: self.debit_taunts.clone(),
        })
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

fn build_svc(wallet_uc: Arc<MockWalletUc>) -> BlackjackService {
    BlackjackService::new(
        Arc::new(FakeBlackjackRepo {
            created: Mutex::new(None),
        }),
        Arc::new(FakeWalletRepo),
        wallet_uc,
    )
}

#[tokio::test]
async fn start_game_propagates_debit_taunts() {
    let mock = Arc::new(MockWalletUc {
        debit_taunts: vec![fake_taunt(StreakKind::EcoBankruptcy)],
        credit_taunts: vec![],
        debit_should_fail: false,
        calls: Mutex::new(vec![]),
    });
    let svc = build_svc(mock.clone());
    let result = svc
        .start_game("g".into(), "u".into(), "x".into(), 50, 10, 1000, 500, 1.5)
        .await
        .expect("start_game ok");
    assert!(result
        .taunt_events
        .iter()
        .any(|e| e.streak_kind == StreakKind::EcoBankruptcy.as_str()));
    assert_eq!(mock.calls.lock().unwrap()[0], "debit");
}

#[tokio::test]
async fn start_game_bubbles_insufficient_funds_error() {
    let mock = Arc::new(MockWalletUc {
        debit_taunts: vec![],
        credit_taunts: vec![],
        debit_should_fail: true,
        calls: Mutex::new(vec![]),
    });
    let svc = build_svc(mock);
    let err = svc
        .start_game("g".into(), "u".into(), "x".into(), 50, 10, 1000, 500, 1.5)
        .await
        .expect_err("doit rejeter si solde insuffisant");
    match err {
        DomainError::ValidationError(msg) => assert!(msg.contains("insuffisant")),
        other => panic!("Expected ValidationError, got {:?}", other),
    }
}

// ── #4 : la transition d'état (compare-and-set) précède le crédit ──

use crate::domain::entities::casino::blackjack::Card;

fn card(rank: &str) -> Card {
    Card {
        rank: rank.into(),
        suit: "spades".into(),
    }
}

/// Repo stateful qui simule le guard SQL `WHERE status='playing'` : `update`
/// ne réussit que si la partie stockée est encore en cours, sinon Conflict.
/// Reproduit fidèlement le compare-and-set du `PgBlackjackRepository`.
struct StatefulBjRepo {
    game: Mutex<BlackjackGame>,
}
#[async_trait]
impl BlackjackRepository for StatefulBjRepo {
    async fn create(&self, _: &BlackjackGame) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_active(&self, _: &str, _: &str) -> Result<Option<BlackjackGame>, DomainError> {
        Ok(None)
    }
    async fn update(&self, game: &BlackjackGame) -> Result<(), DomainError> {
        let mut stored = self.game.lock().unwrap();
        if stored.status != "playing" {
            return Err(DomainError::Conflict(
                "Partie deja terminee ou action concurrente".into(),
            ));
        }
        *stored = game.clone();
        Ok(())
    }
    async fn get_by_id(&self, _: Uuid) -> Result<Option<BlackjackGame>, DomainError> {
        Ok(Some(self.game.lock().unwrap().clone()))
    }
    async fn list_by_guild(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> Result<Vec<BlackjackGame>, DomainError> {
        Ok(vec![])
    }
    async fn cancel_game(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
}

fn winning_playing_game() -> BlackjackGame {
    // Joueur 20, croupier 17, deck vide -> à `stand`, le croupier reste,
    // player_win, payout = bet*2 > 0 (donc un crédit est attendu).
    BlackjackGame {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "x".into(),
        bet: 50,
        player_hand: vec![card("King"), card("Queen")],
        dealer_hand: vec![card("10"), card("7")],
        deck: vec![],
        status: "playing".into(),
        player_score: 20,
        dealer_score: 17,
        doubled: false,
        payout: 0,
        created_at: Utc::now(),
        finished_at: None,
    }
}

fn build_svc_with_repo(
    repo: Arc<StatefulBjRepo>,
    wallet_uc: Arc<MockWalletUc>,
) -> BlackjackService {
    BlackjackService::new(repo, Arc::new(FakeWalletRepo), wallet_uc)
}

#[tokio::test]
async fn stand_resubmitted_on_resolved_game_does_not_recredit() {
    let repo = Arc::new(StatefulBjRepo {
        game: Mutex::new(winning_playing_game()),
    });
    let mock = Arc::new(MockWalletUc {
        debit_taunts: vec![],
        credit_taunts: vec![],
        debit_should_fail: false,
        calls: Mutex::new(vec![]),
    });
    let svc = build_svc_with_repo(repo.clone(), mock.clone());
    let id = repo.game.lock().unwrap().id;

    // 1er stand : remporte la transition, crédite une fois.
    svc.stand(id).await.expect("1er stand ok");
    // 2e stand (rejeu) : la partie n'est plus 'playing' -> Conflict, AUCUN
    // crédit supplémentaire.
    let err = svc.stand(id).await.expect_err("2e stand doit Conflict");
    assert!(matches!(err, DomainError::Conflict(_)));

    let credits = mock
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|c| *c == "credit")
        .count();
    assert_eq!(
        credits, 1,
        "le payout ne doit etre credite qu'une seule fois"
    );
}

#[tokio::test]
async fn concurrent_stands_pay_once() {
    let repo = Arc::new(StatefulBjRepo {
        game: Mutex::new(winning_playing_game()),
    });
    let mock = Arc::new(MockWalletUc {
        debit_taunts: vec![],
        credit_taunts: vec![],
        debit_should_fail: false,
        calls: Mutex::new(vec![]),
    });
    let svc = Arc::new(build_svc_with_repo(repo.clone(), mock.clone()));
    let id = repo.game.lock().unwrap().id;

    // Deux stands "concurrents" : un seul remporte le compare-and-set.
    let s1 = svc.clone();
    let s2 = svc.clone();
    let (r1, r2) = tokio::join!(async move { s1.stand(id).await }, async move {
        s2.stand(id).await
    },);

    let ok = [r1.is_ok(), r2.is_ok()].iter().filter(|x| **x).count();
    assert_eq!(ok, 1, "un seul stand doit réussir");
    let credits = mock
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|c| *c == "credit")
        .count();
    assert_eq!(credits, 1, "un seul crédit malgré deux stands concurrents");
}

#[tokio::test]
async fn start_game_validates_min_max_bet_before_debit() {
    let mock = Arc::new(MockWalletUc {
        debit_taunts: vec![],
        credit_taunts: vec![],
        debit_should_fail: false,
        calls: Mutex::new(vec![]),
    });
    let svc = build_svc(mock.clone());
    let err = svc
        .start_game("g".into(), "u".into(), "x".into(), 5, 10, 1000, 500, 1.5)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
    let err = svc
        .start_game("g".into(), "u".into(), "x".into(), 5000, 10, 1000, 500, 1.5)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
    assert!(mock.calls.lock().unwrap().is_empty());
}
