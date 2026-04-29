use async_trait::async_trait;

use crate::domain::entities::audit::dashboard_stats::DashboardStats;
use crate::domain::entities::audit::user_stats::GuildStatsOverview;
use crate::domain::entities::audit::user_stats::GuildVoiceStats;
use crate::domain::entities::audit::user_stats::UserStats;
use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::ChannelId;

pub struct RecordMessagesCommand {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub count: u64,
}

pub struct RecordVoiceCommand {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub seconds: u64,
    pub channel_id: ChannelId,
    pub channel_name: String,
}

#[async_trait]
pub trait ManageStatsUseCase: Send + Sync {
    async fn record_messages(&self, command: RecordMessagesCommand) -> Result<(), DomainError>;
    async fn record_voice(&self, command: RecordVoiceCommand) -> Result<(), DomainError>;
    async fn get_user_stats(&self, guild_id: &str, user_id: &str) -> Result<Option<UserStats>, DomainError>;
    async fn get_guild_overview(&self, guild_id: &str) -> Result<GuildStatsOverview, DomainError>;
    async fn get_leaderboard(&self, guild_id: &str, limit: u32) -> Result<Vec<UserStats>, DomainError>;
    async fn get_dashboard_stats(&self) -> Result<DashboardStats, DomainError>;
    async fn get_guild_voice_stats(&self, guild_id: &str, days: u32, limit: u32) -> Result<GuildVoiceStats, DomainError>;
}
