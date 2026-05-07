use async_trait::async_trait;

use crate::domain::entities::community::level::LevelConfig;
use crate::domain::entities::community::level::LevelReward;
use crate::domain::entities::community::level::UserLevel;
use crate::domain::entities::community::level::XpSource;
use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;

pub struct SaveLevelConfigCommand {
    pub guild_id: GuildId,
    pub xp_per_message: i32,
    pub xp_per_voice_minute: i32,
    pub xp_cooldown_secs: i32,
    pub level_up_channel_id: Option<String>,
    pub level_up_message: String,
    pub excluded_channels: Vec<String>,
    pub enabled: bool,
}

pub struct AddXpCommand {
    pub guild_id: GuildId,
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

/// Set la valeur exacte de l'XP texte et/ou voix d'un utilisateur.
/// `None` = ne pas modifier ce champ. Les niveaux sont recalcules
/// automatiquement depuis les nouvelles valeurs d'XP.
pub struct SetUserXpCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub xp_text: Option<i64>,
    pub xp_voice: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetTarget {
    All,
    Text,
    Voice,
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
    /// Set valeur exacte XP texte/voix (admin override). Recalcule les niveaux.
    async fn set_user_xp(&self, cmd: SetUserXpCommand) -> Result<UserLevel, DomainError>;
    /// Reset XP a 0 sur la cible (text / voice / all). Recalcule les niveaux.
    async fn reset_user_xp(&self, guild_id: &str, user_id: &str, target: ResetTarget) -> Result<UserLevel, DomainError>;
}
