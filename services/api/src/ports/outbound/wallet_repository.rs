use async_trait::async_trait;

use crate::domain::entities::{Wallet, WalletTransaction};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait WalletRepository: Send + Sync {
    async fn get_or_create(&self, guild_id: &str, user_id: &str, username: &str, starting_coins: i64) -> Result<Wallet, DomainError>;
    async fn get(&self, guild_id: &str, user_id: &str) -> Result<Option<Wallet>, DomainError>;
    async fn credit(&self, guild_id: &str, user_id: &str, amount: i64, source: &str, description: &str) -> Result<Wallet, DomainError>;
    async fn debit(&self, guild_id: &str, user_id: &str, amount: i64, source: &str, description: &str) -> Result<Wallet, DomainError>;
    async fn transfer(&self, guild_id: &str, from_user: &str, to_user: &str, amount: i64, source: &str, description: &str) -> Result<(), DomainError>;
    async fn leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<Wallet>, DomainError>;
    async fn get_transactions(&self, guild_id: &str, user_id: &str, limit: i64) -> Result<Vec<WalletTransaction>, DomainError>;

    /// Liste tous les wallets d'un serveur (page wallet du desktop).
    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Wallet>, DomainError>;
    /// Remet un wallet individuel a `new_balance` coins et efface son historique.
    async fn reset_wallet(&self, guild_id: &str, user_id: &str, new_balance: i64) -> Result<Wallet, DomainError>;
    /// Reset tous les wallets d'un serveur a `new_balance`. Retourne le nombre de comptes reset.
    async fn reset_all_wallets(&self, guild_id: &str, new_balance: i64) -> Result<u64, DomainError>;
}
