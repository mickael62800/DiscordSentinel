use std::sync::Arc;

use crate::domain::entities::Wallet;
use crate::domain::ports::WalletRepository;

pub struct WalletService {
    repo: Arc<dyn WalletRepository>,
}

impl WalletService {
    pub fn new(repo: Arc<dyn WalletRepository>) -> Self {
        Self { repo }
    }

    pub async fn list(&self, guild_id: String) -> Result<Vec<Wallet>, String> {
        self.repo.list_wallets(guild_id).await
    }

    pub async fn credit(&self, guild_id: String, user_id: String, amount: i64, description: String) -> Result<Wallet, String> {
        self.repo.credit_wallet(guild_id, user_id, amount, description).await
    }

    pub async fn debit(&self, guild_id: String, user_id: String, amount: i64, description: String) -> Result<Wallet, String> {
        self.repo.debit_wallet(guild_id, user_id, amount, description).await
    }

    pub async fn reset(&self, guild_id: String, user_id: String, new_balance: i64) -> Result<Wallet, String> {
        self.repo.reset_wallet(guild_id, user_id, new_balance).await
    }

    pub async fn reset_all(&self, guild_id: String, new_balance: i64) -> Result<u64, String> {
        self.repo.reset_all_wallets(guild_id, new_balance).await
    }
}
