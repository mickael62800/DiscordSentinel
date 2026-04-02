use async_trait::async_trait;

use crate::domain::entities::{ModerationAction, UserModerationHistory};
use crate::domain::errors::DomainError;

pub struct LogModerationCommand {
    pub guild_id: String,
    pub channel_id: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub gravity: Option<String>,
    pub duration: Option<u64>,
}

#[async_trait]
pub trait ManageModerationUseCase: Send + Sync {
    async fn log_action(&self, command: LogModerationCommand) -> Result<ModerationAction, DomainError>;
    async fn get_history(&self, guild_id: &str, target_id: &str) -> Result<UserModerationHistory, DomainError>;
    async fn list_bans(&self, guild_id: Option<&str>, limit: i64, offset: i64) -> Result<Vec<ModerationAction>, DomainError>;
    async fn delete_bans_for_user(&self, guild_id: &str, target_id: &str) -> Result<(), DomainError>;
}
