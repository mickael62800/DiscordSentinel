use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::community::voice_channel::VoiceChannelBan;
use crate::domain::entities::community::voice_channel::VoiceChannelWhitelistEntry;
use crate::domain::errors::DomainError;
use crate::ports::inbound::community::manage_voice_channels::BanFromChannelCommand;
use crate::ports::inbound::community::manage_voice_channels::ManageWhitelistCommand;
use super::ManageVoiceChannelsService;

impl ManageVoiceChannelsService {
    pub(super) async fn get_whitelist_impl(&self, guild_id: &str, owner_id: &str) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError> {
        self.repo.find_whitelist(guild_id, owner_id).await
    }

    pub(super) async fn add_to_whitelist_impl(&self, cmd: ManageWhitelistCommand) -> Result<(), DomainError> {
        let entry = VoiceChannelWhitelistEntry {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            owner_id: cmd.owner_id,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            created_at: Utc::now(),
        };

        self.repo.add_to_whitelist(&entry).await
    }

    pub(super) async fn remove_from_whitelist_impl(&self, guild_id: &str, owner_id: &str, target_id: &str) -> Result<(), DomainError> {
        self.repo.remove_from_whitelist(guild_id, owner_id, target_id).await
    }

    pub(super) async fn ban_from_channel_impl(&self, cmd: BanFromChannelCommand) -> Result<(), DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;

        let expires_at = cmd.duration_secs.map(|secs| Utc::now() + chrono::Duration::seconds(secs));

        let ban = VoiceChannelBan {
            id: Uuid::new_v4(),
            voice_channel_id: channel.id,
            user_id: cmd.user_id,
            user_name: cmd.user_name,
            banned_by: cmd.banned_by,
            reason: cmd.reason,
            expires_at,
            created_at: Utc::now(),
        };

        self.repo.save_ban(&ban).await?;
        self.invalidate_cache(&channel.guild_id, &cmd.channel_id).await;
        Ok(())
    }

    pub(super) async fn unban_from_channel_impl(&self, channel_id: &str, user_id: &str) -> Result<(), DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        self.repo.remove_ban(channel.id, user_id).await?;
        self.invalidate_cache(&channel.guild_id, channel_id).await;
        Ok(())
    }

    pub(super) async fn is_banned_impl(&self, channel_id: &str, user_id: &str) -> Result<bool, DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        let ban = self.repo.find_active_ban(channel.id, user_id).await?;
        Ok(ban.is_some())
    }
}
