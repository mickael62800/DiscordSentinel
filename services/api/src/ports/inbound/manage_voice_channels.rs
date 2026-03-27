use async_trait::async_trait;

use crate::domain::entities::{VoiceChannel, VoiceChannelDetail, VoiceChannelWhitelistEntry};
use crate::domain::errors::DomainError;

pub struct CreateVoiceChannelCommand {
    pub guild_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub channel_id: String,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    pub kind: String,
    pub visibility: String,
    pub queue_enabled: bool,
}

pub struct UpdateVoiceChannelCommand {
    pub channel_id: String,
    pub visibility: Option<String>,
    pub locked: Option<bool>,
    pub queue_enabled: Option<bool>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub member_limit: Option<Option<i32>>,
    pub queue_channel_id: Option<Option<String>>,
}

pub struct TransferOwnershipCommand {
    pub channel_id: String,
    pub new_owner_id: String,
    pub new_owner_name: String,
}

pub struct ManageCoAdminCommand {
    pub channel_id: String,
    pub user_id: String,
    pub user_name: String,
}

pub struct ManageWhitelistCommand {
    pub guild_id: String,
    pub owner_id: String,
    pub target_id: String,
    pub target_name: String,
}

pub struct BanFromChannelCommand {
    pub channel_id: String,
    pub user_id: String,
    pub user_name: String,
    pub banned_by: String,
    pub reason: Option<String>,
    pub duration_secs: Option<i64>,
}

#[async_trait]
pub trait ManageVoiceChannelsUseCase: Send + Sync {
    async fn list_all_channels(&self) -> Result<Vec<VoiceChannel>, DomainError>;
    async fn list_channels(&self, guild_id: &str) -> Result<Vec<VoiceChannel>, DomainError>;
    async fn get_channel_detail(&self, channel_id: &str) -> Result<VoiceChannelDetail, DomainError>;
    async fn create_channel(&self, cmd: CreateVoiceChannelCommand) -> Result<VoiceChannel, DomainError>;
    async fn close_channel(&self, channel_id: &str) -> Result<(), DomainError>;
    async fn delete_channel(&self, channel_id: &str) -> Result<(), DomainError>;
    async fn update_channel(&self, cmd: UpdateVoiceChannelCommand) -> Result<(), DomainError>;
    async fn transfer_ownership(&self, cmd: TransferOwnershipCommand) -> Result<(), DomainError>;

    // Co-admins
    async fn add_co_admin(&self, cmd: ManageCoAdminCommand) -> Result<(), DomainError>;
    async fn remove_co_admin(&self, channel_id: &str, user_id: &str) -> Result<(), DomainError>;

    // Whitelist
    async fn get_whitelist(&self, guild_id: &str, owner_id: &str) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError>;
    async fn add_to_whitelist(&self, cmd: ManageWhitelistCommand) -> Result<(), DomainError>;
    async fn remove_from_whitelist(&self, guild_id: &str, owner_id: &str, target_id: &str) -> Result<(), DomainError>;

    // Bans
    async fn ban_from_channel(&self, cmd: BanFromChannelCommand) -> Result<(), DomainError>;
    async fn unban_from_channel(&self, channel_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn is_banned(&self, channel_id: &str, user_id: &str) -> Result<bool, DomainError>;
}
