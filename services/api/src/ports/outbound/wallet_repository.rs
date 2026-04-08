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
}
