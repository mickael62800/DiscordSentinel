//! Tests d'integration HTTP pour les endpoints wallet.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::ports::outbound::casino::wallet_repository::WalletRepository;
use sentinel_core::domain::entities::casino::wallet::Wallet;
use sentinel_core::domain::entities::casino::wallet::WalletTransaction;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::uow::DbTx;

use test_helpers::build_test_state_wallet;

// ══════════════════════════════════════════════════════════
// Mock WalletRepository
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockWalletRepo {
    wallets: Mutex<Vec<Wallet>>,
    txs: Mutex<Vec<WalletTransaction>>,
    transfers: Mutex<Vec<(String, String, String, i64)>>, // (guild, from, to, amount)
    resets: Mutex<Vec<(String, String, i64)>>,
    bulk_resets: Mutex<Vec<(String, i64, u64)>>,
}

impl MockWalletRepo {
    fn new() -> Self {
        Self::default()
    }
    fn with(self, w: Wallet) -> Self {
        self.wallets.lock().unwrap().push(w);
        self
    }
}

fn sample_wallet(guild_id: &str, user_id: &str, coins: i64) -> Wallet {
    Wallet {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        user_id: user_id.into(),
        username: user_id.into(),
        coins,
        total_earned: coins.max(0),
        total_spent: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[async_trait]
impl WalletRepository for MockWalletRepo {
    async fn get_or_create(
        &self,
        guild_id: &str,
        user_id: &str,
        _username: &str,
        starting_coins: i64,
    ) -> Result<Wallet, DomainError> {
        let mut wallets = self.wallets.lock().unwrap();
        if let Some(w) = wallets
            .iter()
            .find(|w| w.guild_id.as_str() == guild_id && w.user_id.as_str() == user_id)
        {
            return Ok(w.clone());
        }
        let w = sample_wallet(guild_id, user_id, starting_coins);
        wallets.push(w.clone());
        Ok(w)
    }
    async fn get(&self, guild_id: &str, user_id: &str) -> Result<Option<Wallet>, DomainError> {
        Ok(self
            .wallets
            .lock()
            .unwrap()
            .iter()
            .find(|w| w.guild_id.as_str() == guild_id && w.user_id.as_str() == user_id)
            .cloned())
    }
    async fn credit(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        _: &str,
        _: &str,
    ) -> Result<Wallet, DomainError> {
        let mut wallets = self.wallets.lock().unwrap();
        let w = wallets
            .iter_mut()
            .find(|w| w.guild_id.as_str() == guild_id && w.user_id.as_str() == user_id);
        match w {
            Some(w) => {
                w.coins += amount;
                w.total_earned += amount;
                Ok(w.clone())
            }
            None => {
                let new = sample_wallet(guild_id, user_id, amount);
                wallets.push(new.clone());
                Ok(new)
            }
        }
    }
    async fn debit(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        _: &str,
        _: &str,
    ) -> Result<Wallet, DomainError> {
        let mut wallets = self.wallets.lock().unwrap();
        let w = wallets
            .iter_mut()
            .find(|w| w.guild_id.as_str() == guild_id && w.user_id.as_str() == user_id)
            .ok_or_else(|| DomainError::NotFound("wallet".into()))?;
        if w.coins < amount {
            return Err(DomainError::ValidationError("solde insuffisant".into()));
        }
        w.coins -= amount;
        w.total_spent += amount;
        Ok(w.clone())
    }
    async fn transfer(
        &self,
        guild_id: &str,
        from: &str,
        to: &str,
        amount: i64,
        _: &str,
        _: &str,
    ) -> Result<(), DomainError> {
        self.transfers
            .lock()
            .unwrap()
            .push((guild_id.into(), from.into(), to.into(), amount));
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
    async fn leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<Wallet>, DomainError> {
        let wallets = self.wallets.lock().unwrap();
        let mut matching: Vec<Wallet> = wallets
            .iter()
            .filter(|w| w.guild_id.as_str() == guild_id)
            .cloned()
            .collect();
        matching.sort_by(|a, b| b.coins.cmp(&a.coins));
        matching.truncate(limit as usize);
        Ok(matching)
    }
    async fn get_transactions(
        &self,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Vec<WalletTransaction>, DomainError> {
        Ok(self.txs.lock().unwrap().clone())
    }
    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Wallet>, DomainError> {
        Ok(self
            .wallets
            .lock()
            .unwrap()
            .iter()
            .filter(|w| w.guild_id.as_str() == guild_id)
            .cloned()
            .collect())
    }
    async fn reset_wallet(
        &self,
        guild_id: &str,
        user_id: &str,
        new_balance: i64,
    ) -> Result<Wallet, DomainError> {
        self.resets
            .lock()
            .unwrap()
            .push((guild_id.into(), user_id.into(), new_balance));
        let mut wallets = self.wallets.lock().unwrap();
        let w = wallets
            .iter_mut()
            .find(|w| w.guild_id.as_str() == guild_id && w.user_id.as_str() == user_id);
        match w {
            Some(w) => {
                w.coins = new_balance;
                w.total_earned = 0;
                w.total_spent = 0;
                Ok(w.clone())
            }
            None => {
                let new = sample_wallet(guild_id, user_id, new_balance);
                wallets.push(new.clone());
                Ok(new)
            }
        }
    }
    async fn reset_all_wallets(
        &self,
        guild_id: &str,
        new_balance: i64,
    ) -> Result<u64, DomainError> {
        let mut wallets = self.wallets.lock().unwrap();
        let mut affected = 0u64;
        for w in wallets
            .iter_mut()
            .filter(|w| w.guild_id.as_str() == guild_id)
        {
            w.coins = new_balance;
            affected += 1;
        }
        self.bulk_resets
            .lock()
            .unwrap()
            .push((guild_id.into(), new_balance, affected));
        Ok(affected)
    }
    async fn credit_in_tx(
        &self,
        _tx: &mut dyn DbTx,
        _guild_id: &str,
        _user_id: &str,
        _amount: i64,
        _source: &str,
        _description: &str,
    ) -> Result<(i64, i64), DomainError> {
        unimplemented!()
    }
    async fn debit_in_tx(
        &self,
        _tx: &mut dyn DbTx,
        _guild_id: &str,
        _user_id: &str,
        _amount: i64,
        _source: &str,
        _description: &str,
    ) -> Result<(i64, i64), DomainError> {
        unimplemented!()
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_app(repo: Arc<MockWalletRepo>) -> axum::Router {
    router::build_for_test(build_test_state_wallet(repo))
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

async fn post_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

// ══════════════════════════════════════════════════════════
// GET /api/wallet/{guild_id}/{user_id}
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_wallet_creates_with_default_starting_coins() {
    unsafe { std::env::remove_var("WALLET_STARTING_COINS") };
    let repo = Arc::new(MockWalletRepo::new());
    let app = build_app(repo.clone());
    let (status, json) = get(app, "/api/wallet/111111111111111111/444444444444444444").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["coins"], 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_wallet_invalid_guild_id_422() {
    let app = build_app(Arc::new(MockWalletRepo::new()));
    let (status, _) = get(app, "/api/wallet/bad/444444444444444444").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_wallet_returns_existing() {
    let repo = Arc::new(MockWalletRepo::new().with(sample_wallet(
        "111111111111111111",
        "444444444444444444",
        500,
    )));
    let app = build_app(repo);
    let (status, json) = get(app, "/api/wallet/111111111111111111/444444444444444444").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["coins"], 500);
}

// ══════════════════════════════════════════════════════════
// POST credit / debit
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credit_success() {
    let repo = Arc::new(MockWalletRepo::new());
    let app = build_app(repo.clone());
    let body = serde_json::json!({
        "amount": 50, "source": "reward", "description": "daily"
    });
    let (status, json) = post_json(
        app,
        "/api/wallet/111111111111111111/444444444444444444/credit",
        body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["coins"], 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credit_zero_amount_422() {
    let app = build_app(Arc::new(MockWalletRepo::new()));
    let body = serde_json::json!({"amount": 0, "source": "s", "description": "d"});
    let (status, json) = post_json(
        app,
        "/api/wallet/111111111111111111/444444444444444444/credit",
        body,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("positif"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credit_negative_amount_422() {
    let app = build_app(Arc::new(MockWalletRepo::new()));
    let body = serde_json::json!({"amount": -10, "source": "s", "description": "d"});
    let (status, _) = post_json(
        app,
        "/api/wallet/111111111111111111/444444444444444444/credit",
        body,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debit_success() {
    let repo = Arc::new(MockWalletRepo::new().with(sample_wallet(
        "111111111111111111",
        "444444444444444444",
        200,
    )));
    let app = build_app(repo);
    let body = serde_json::json!({"amount": 50, "source": "buy", "description": "item"});
    let (status, json) = post_json(
        app,
        "/api/wallet/111111111111111111/444444444444444444/debit",
        body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["coins"], 150);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debit_insufficient_funds_fails() {
    let repo = Arc::new(MockWalletRepo::new().with(sample_wallet(
        "111111111111111111",
        "444444444444444444",
        10,
    )));
    let app = build_app(repo);
    let body = serde_json::json!({"amount": 100, "source": "s", "description": "d"});
    let (status, _) = post_json(
        app,
        "/api/wallet/111111111111111111/444444444444444444/debit",
        body,
    )
    .await;
    assert!(status.is_client_error());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn debit_zero_amount_422() {
    let app = build_app(Arc::new(MockWalletRepo::new()));
    let body = serde_json::json!({"amount": 0, "source": "s", "description": "d"});
    let (status, _) = post_json(
        app,
        "/api/wallet/111111111111111111/444444444444444444/debit",
        body,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// POST /api/wallet/transfer
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_success() {
    let repo = Arc::new(MockWalletRepo::new());
    let app = build_app(repo.clone());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "from_user_id": "444444444444444444",
        "to_user_id": "555555555555555555",
        "amount": 30, "source": "gift", "description": "cadeau"
    });
    let (status, json) = post_json(app, "/api/wallet/transfer", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    let tfers = repo.transfers.lock().unwrap();
    assert_eq!(tfers.len(), 1);
    assert_eq!(tfers[0].3, 30);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_self_transfer_rejected() {
    let app = build_app(Arc::new(MockWalletRepo::new()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "from_user_id": "444444444444444444",
        "to_user_id": "444444444444444444",
        "amount": 10, "source": "s", "description": "d"
    });
    let (status, json) = post_json(app, "/api/wallet/transfer", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("soi-meme"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_zero_amount_422() {
    let app = build_app(Arc::new(MockWalletRepo::new()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "from_user_id": "444444444444444444",
        "to_user_id": "555555555555555555",
        "amount": 0, "source": "s", "description": "d"
    });
    let (status, _) = post_json(app, "/api/wallet/transfer", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_invalid_guild_422() {
    let app = build_app(Arc::new(MockWalletRepo::new()));
    let body = serde_json::json!({
        "guild_id": "bad",
        "from_user_id": "444444444444444444",
        "to_user_id": "555555555555555555",
        "amount": 10, "source": "s", "description": "d"
    });
    let (status, _) = post_json(app, "/api/wallet/transfer", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// Leaderboard / transactions / list_wallets
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_sorted_desc() {
    let repo = Arc::new(
        MockWalletRepo::new()
            .with(sample_wallet("111111111111111111", "u1", 50))
            .with(sample_wallet("111111111111111111", "u2", 200))
            .with(sample_wallet("111111111111111111", "u3", 100)),
    );
    let app = build_app(repo);
    let (status, json) = get(app, "/api/wallet/111111111111111111/leaderboard").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["user_id"], "u2");
    assert_eq!(arr[2]["user_id"], "u1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_respects_limit() {
    let mut repo = MockWalletRepo::new();
    for i in 0..10 {
        repo = repo.with(sample_wallet(
            "111111111111111111",
            &format!("u{i}"),
            i * 10,
        ));
    }
    let app = build_app(Arc::new(repo));
    let (status, json) = get(app, "/api/wallet/111111111111111111/leaderboard?limit=3").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transactions_empty() {
    let app = build_app(Arc::new(MockWalletRepo::new()));
    let (status, json) = get(
        app,
        "/api/wallet/111111111111111111/444444444444444444/transactions",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_wallets_scoped_to_guild() {
    let repo = Arc::new(
        MockWalletRepo::new()
            .with(sample_wallet("111111111111111111", "u1", 100))
            .with(sample_wallet("222222222222222222", "u2", 200)),
    );
    let app = build_app(repo);
    let (status, json) = get(app, "/api/wallets/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
}

// ══════════════════════════════════════════════════════════
// Reset
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_wallet_default_100() {
    let repo = Arc::new(MockWalletRepo::new().with(sample_wallet(
        "111111111111111111",
        "444444444444444444",
        500,
    )));
    let app = build_app(repo.clone());
    let body = serde_json::json!({});
    let (status, json) = post_json(
        app,
        "/api/wallets/111111111111111111/444444444444444444/reset",
        body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["coins"], 100);
    let resets = repo.resets.lock().unwrap();
    assert_eq!(resets.len(), 1);
    assert_eq!(resets[0].2, 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_wallet_custom_balance() {
    let repo = Arc::new(MockWalletRepo::new());
    let app = build_app(repo.clone());
    let body = serde_json::json!({"new_balance": 500});
    let (status, json) = post_json(
        app,
        "/api/wallets/111111111111111111/444444444444444444/reset",
        body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["coins"], 500);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_wallet_negative_clamped_to_zero() {
    let repo = Arc::new(MockWalletRepo::new());
    let app = build_app(repo.clone());
    let body = serde_json::json!({"new_balance": -50});
    let (status, json) = post_json(
        app,
        "/api/wallets/111111111111111111/444444444444444444/reset",
        body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // Le domain clamp a 0.
    assert_eq!(json["coins"], 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_all_wallets_default_balance() {
    let repo = Arc::new(
        MockWalletRepo::new()
            .with(sample_wallet("111111111111111111", "u1", 500))
            .with(sample_wallet("111111111111111111", "u2", 1000)),
    );
    let app = build_app(repo.clone());
    let body = serde_json::json!({});
    let (status, json) = post_json(app, "/api/wallets/111111111111111111/reset-all", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["affected"], 2);
    assert_eq!(json["new_balance"], 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reset_all_wallets_invalid_guild_422() {
    let app = build_app(Arc::new(MockWalletRepo::new()));
    let body = serde_json::json!({"new_balance": 50});
    let (status, _) = post_json(app, "/api/wallets/bad/reset-all", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
