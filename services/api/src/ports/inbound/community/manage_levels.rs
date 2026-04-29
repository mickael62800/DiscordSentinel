use async_trait::async_trait;

use crate::domain::entities::community::level::LevelConfig;
use crate::domain::entities::community::level::LevelReward;
use crate::domain::entities::community::level::UserLevel;
use crate::domain::entities::community::level::XpSource;
use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::UserId;

pub struct SaveLevelConfigCommand {
    pub guild_id: String,
    pub xp_per_message: i32,
    pub xp_per_voice_minute: i32,
    pub xp_cooldown_secs: i32,
    pub level_up_channel_id: Option<String>,
    pub level_up_message: String,
    pub excluded_channels: Vec<String>,
    pub enabled: bool,
}

pub struct AddXpCommand {
    pub guild_id: String,
    pub user_id: UserId,
    pub username: String,
    pub amount: i64,
    pub source: XpSource,
}

pub struct AddXpResult {
    pub user_level: UserLevel,
    pub leveled_up: bool,
    pub old_level: i32,
    pub reward_role_id: Option<String>,
    pub source: XpSource,
}

#[async_trait]
pub trait ManageLevelsUseCase: Send + Sync {
    async fn get_config(&self, guild_id: &str) -> Result<LevelConfig, DomainError>;
    async fn save_config(&self, cmd: SaveLevelConfigCommand) -> Result<LevelConfig, DomainError>;
    async fn add_xp(&self, cmd: AddXpCommand) -> Result<AddXpResult, DomainError>;
    async fn get_user_level(&self, guild_id: &str, user_id: &str) -> Result<UserLevel, DomainError>;
    async fn get_leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<UserLevel>, DomainError>;
    async fn get_leaderboard_by_source(&self, guild_id: &str, source: XpSource, limit: i64) -> Result<Vec<UserLevel>, DomainError>;
    async fn get_rewards(&self, guild_id: &str) -> Result<Vec<LevelReward>, DomainError>;
    async fn get_rewards_by_source(&self, guild_id: &str, source: XpSource) -> Result<Vec<LevelReward>, DomainError>;
    async fn set_reward(&self, guild_id: &str, level: i32, role_id: &str, source: XpSource) -> Result<LevelReward, DomainError>;
    async fn delete_reward(&self, guild_id: &str, level: i32, source: XpSource) -> Result<(), DomainError>;
}
