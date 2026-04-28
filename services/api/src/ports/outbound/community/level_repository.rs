use async_trait::async_trait;

use crate::domain::entities::{LevelConfig, LevelReward, UserLevel, XpSource};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait LevelRepository: Send + Sync {
    async fn get_config(&self, guild_id: &str) -> Result<Option<LevelConfig>, DomainError>;
    async fn upsert_config(&self, config: &LevelConfig) -> Result<(), DomainError>;
    async fn get_user_level(&self, guild_id: &str, user_id: &str) -> Result<Option<UserLevel>, DomainError>;
    async fn upsert_user_level(&self, user: &UserLevel) -> Result<(), DomainError>;
    /// Ajoute de l'XP de maniere atomique (pas de race condition).
    /// Retourne le user_level mis a jour.
    async fn add_xp_atomic(&self, guild_id: &str, user_id: &str, username: &str, amount: i64, source: XpSource) -> Result<UserLevel, DomainError>;
    async fn get_leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<UserLevel>, DomainError>;
    async fn get_leaderboard_by_source(&self, guild_id: &str, source: XpSource, limit: i64) -> Result<Vec<UserLevel>, DomainError>;
    async fn get_rewards(&self, guild_id: &str) -> Result<Vec<LevelReward>, DomainError>;
    async fn get_rewards_by_source(&self, guild_id: &str, source: XpSource) -> Result<Vec<LevelReward>, DomainError>;
    async fn upsert_reward(&self, reward: &LevelReward) -> Result<(), DomainError>;
    async fn delete_reward(&self, guild_id: &str, level: i32, source: XpSource) -> Result<(), DomainError>;
}
