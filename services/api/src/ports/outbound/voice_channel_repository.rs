use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{VoiceChannel, VoiceChannelBan, VoiceChannelCoAdmin, VoiceChannelInviteLink, VoiceChannelTheme, VoiceChannelWhitelistEntry};
use crate::domain::errors::DomainError;

#[async_trait]
#[allow(dead_code)]
pub trait VoiceChannelRepository: Send + Sync {
    // Channels
    async fn find_all(&self) -> Result<Vec<VoiceChannel>, DomainError>;
    async fn find_all_by_guild(&self, guild_id: &str) -> Result<Vec<VoiceChannel>, DomainError>;
    async fn find_by_channel_id(&self, channel_id: &str) -> Result<Option<VoiceChannel>, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<VoiceChannel>, DomainError>;
    async fn save(&self, channel: &VoiceChannel) -> Result<(), DomainError>;
    async fn close(&self, id: Uuid) -> Result<(), DomainError>;
    async fn close_by_channel_id(&self, channel_id: &str) -> Result<(), DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    async fn update_visibility(&self, id: Uuid, visibility: &str) -> Result<(), DomainError>;
    async fn update_locked(&self, id: Uuid, locked: bool) -> Result<(), DomainError>;
    async fn update_queue_enabled(&self, id: Uuid, queue_enabled: bool) -> Result<(), DomainError>;
    async fn update_name(&self, id: Uuid, name: &str) -> Result<(), DomainError>;
    async fn update_status(&self, id: Uuid, status: Option<&str>) -> Result<(), DomainError>;
    async fn update_member_limit(&self, id: Uuid, limit: Option<i32>) -> Result<(), DomainError>;
    async fn update_owner(&self, id: Uuid, owner_id: &str, owner_name: &str) -> Result<(), DomainError>;
    async fn update_queue_channel(&self, id: Uuid, queue_channel_id: Option<&str>) -> Result<(), DomainError>;
    async fn update_stage(&self, id: Uuid, stage_enabled: bool) -> Result<(), DomainError>;

    // Co-admins
    async fn find_co_admins(&self, voice_channel_id: Uuid) -> Result<Vec<VoiceChannelCoAdmin>, DomainError>;
    async fn add_co_admin(&self, co_admin: &VoiceChannelCoAdmin) -> Result<(), DomainError>;
    async fn remove_co_admin(&self, voice_channel_id: Uuid, user_id: &str) -> Result<(), DomainError>;

    // Whitelists
    async fn find_whitelist(&self, guild_id: &str, owner_id: &str) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError>;
    async fn add_to_whitelist(&self, entry: &VoiceChannelWhitelistEntry) -> Result<(), DomainError>;
    async fn remove_from_whitelist(&self, guild_id: &str, owner_id: &str, target_id: &str) -> Result<(), DomainError>;

    // Bans
    async fn find_bans(&self, voice_channel_id: Uuid) -> Result<Vec<VoiceChannelBan>, DomainError>;
    async fn find_active_ban(&self, voice_channel_id: Uuid, user_id: &str) -> Result<Option<VoiceChannelBan>, DomainError>;
    async fn save_ban(&self, ban: &VoiceChannelBan) -> Result<(), DomainError>;
    async fn remove_ban(&self, voice_channel_id: Uuid, user_id: &str) -> Result<(), DomainError>;
    async fn cleanup_expired_bans(&self) -> Result<u64, DomainError>;

    // Invite Links
    async fn find_invite_links(&self, voice_channel_id: Uuid) -> Result<Vec<VoiceChannelInviteLink>, DomainError>;
    async fn find_invite_by_code(&self, code: &str) -> Result<Option<VoiceChannelInviteLink>, DomainError>;
    async fn save_invite_link(&self, link: &VoiceChannelInviteLink) -> Result<(), DomainError>;
    async fn increment_invite_uses(&self, id: Uuid) -> Result<bool, DomainError>;
    async fn revoke_invite_link(&self, id: Uuid) -> Result<(), DomainError>;

    // Themes
    async fn find_themes(&self, guild_id: &str) -> Result<Vec<VoiceChannelTheme>, DomainError>;
    async fn find_theme(&self, id: Uuid) -> Result<Option<VoiceChannelTheme>, DomainError>;
    async fn save_theme(&self, theme: &VoiceChannelTheme) -> Result<(), DomainError>;
    async fn update_theme(&self, theme: &VoiceChannelTheme) -> Result<(), DomainError>;
    async fn delete_theme(&self, id: Uuid) -> Result<(), DomainError>;
    async fn clear_default_themes(&self, guild_id: &str) -> Result<(), DomainError>;
}
