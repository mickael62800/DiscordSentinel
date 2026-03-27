use async_trait::async_trait;

use crate::domain::entities::{LevelConfig, LevelReward, UserLevel};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait LevelRepository: Send + Sync {
    async fn get_config(&self, guild_id: &str) -> Result<Option<LevelConfig>, DomainError>;
    async fn upsert_config(&self, config: &LevelConfig) -> Result<(), DomainError>;
    async fn get_user_level(&self, guild_id: &str, user_id: &str) -> Result<Option<UserLevel>, DomainError>;
    async fn upsert_user_level(&self, user: &UserLevel) -> Result<(), DomainError>;
    async fn get_leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<UserLevel>, DomainError>;
    async fn get_rewards(&self, guild_id: &str) -> Result<Vec<LevelReward>, DomainError>;
    async fn upsert_reward(&self, reward: &LevelReward) -> Result<(), DomainError>;
    async fn delete_reward(&self, guild_id: &str, level: i32) -> Result<(), DomainError>;
}
