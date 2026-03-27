use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::{
    VoiceChannel, VoiceChannelBan, VoiceChannelCoAdmin, VoiceChannelDetail,
    VoiceChannelWhitelistEntry,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::{
    BanFromChannelCommand, CreateVoiceChannelCommand, ManageCoAdminCommand,
    ManageVoiceChannelsUseCase, ManageWhitelistCommand, TransferOwnershipCommand,
    UpdateVoiceChannelCommand,
};
use crate::ports::outbound::{CachePort, VoiceChannelRepository};

const CHANNELS_LIST_TTL: u64 = 60;
const CHANNEL_DETAIL_TTL: u64 = 300;

pub struct ManageVoiceChannelsService {
    repo: Arc<dyn VoiceChannelRepository>,
    cache: Arc<dyn CachePort>,
}

impl ManageVoiceChannelsService {
    pub fn new(repo: Arc<dyn VoiceChannelRepository>, cache: Arc<dyn CachePort>) -> Self {
        Self { repo, cache }
    }

    async fn invalidate_cache(&self, guild_id: &str, channel_id: &str) {
        self.cache.invalidate(&format!("voice_channels:{guild_id}")).await.ok();
        self.cache.invalidate(&format!("voice_channel:{channel_id}")).await.ok();
    }

    async fn resolve_channel(&self, channel_id: &str) -> Result<VoiceChannel, DomainError> {
        self.repo
            .find_by_channel_id(channel_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Salon vocal introuvable : {channel_id}")))
    }
}

#[async_trait]
impl ManageVoiceChannelsUseCase for ManageVoiceChannelsService {
    async fn list_all_channels(&self) -> Result<Vec<VoiceChannel>, DomainError> {
        self.repo.find_all().await
    }

    async fn list_channels(&self, guild_id: &str) -> Result<Vec<VoiceChannel>, DomainError> {
        let cache_key = format!("voice_channels:{guild_id}");

        if let Some(json) = self.cache.get_json(&cache_key).await? {
            if let Ok(channels) = serde_json::from_str::<Vec<VoiceChannel>>(&json) {
                return Ok(channels);
            }
        }

        let channels = self.repo.find_all_by_guild(guild_id).await?;

        if let Ok(json) = serde_json::to_string(&channels) {
            self.cache.set_json(&cache_key, &json, CHANNELS_LIST_TTL).await.ok();
        }

        Ok(channels)
    }

    async fn get_channel_detail(&self, channel_id: &str) -> Result<VoiceChannelDetail, DomainError> {
        let cache_key = format!("voice_channel:{channel_id}");

        if let Some(json) = self.cache.get_json(&cache_key).await? {
            if let Ok(detail) = serde_json::from_str::<VoiceChannelDetail>(&json) {
                return Ok(detail);
            }
        }

        let channel = self.resolve_channel(channel_id).await?;
        let co_admins = self.repo.find_co_admins(channel.id).await?;
        let bans = self.repo.find_bans(channel.id).await?;

        let detail = VoiceChannelDetail { channel, co_admins, bans };

        if let Ok(json) = serde_json::to_string(&detail) {
            self.cache.set_json(&cache_key, &json, CHANNEL_DETAIL_TTL).await.ok();
        }

        Ok(detail)
    }

    async fn create_channel(&self, cmd: CreateVoiceChannelCommand) -> Result<VoiceChannel, DomainError> {
        let channel = VoiceChannel {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            owner_id: cmd.owner_id,
            owner_name: cmd.owner_name,
            channel_id: cmd.channel_id,
            text_channel_id: cmd.text_channel_id,
            members_channel_id: cmd.members_channel_id,
            queue_channel_id: cmd.queue_channel_id,
            category_id: cmd.category_id,
            channel_name: cmd.channel_name,
            kind: cmd.kind,
            visibility: cmd.visibility,
            queue_enabled: cmd.queue_enabled,
            locked: false,
            member_limit: None,
            status: None,
            channel_status: "open".to_string(),
            closed_at: None,
            created_at: Utc::now(),
        };

        self.repo.save(&channel).await?;
        self.cache.invalidate(&format!("voice_channels:{}", channel.guild_id)).await.ok();

        Ok(channel)
    }

    async fn close_channel(&self, channel_id: &str) -> Result<(), DomainError> {
        self.repo.close_by_channel_id(channel_id).await?;
        // Invalider le cache — on essaie de résoudre le channel pour le guild_id
        if let Ok(channel) = self.resolve_channel(channel_id).await {
            self.invalidate_cache(&channel.guild_id, channel_id).await;
        }
        Ok(())
    }

    async fn delete_channel(&self, channel_id: &str) -> Result<(), DomainError> {
        // Soft-delete : close au lieu de supprimer
        self.close_channel(channel_id).await
    }

    async fn update_channel(&self, cmd: UpdateVoiceChannelCommand) -> Result<(), DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;

        if let Some(vis) = &cmd.visibility {
            self.repo.update_visibility(channel.id, vis).await?;
        }
        if let Some(locked) = cmd.locked {
            self.repo.update_locked(channel.id, locked).await?;
        }
        if let Some(queue_enabled) = cmd.queue_enabled {
            self.repo.update_queue_enabled(channel.id, queue_enabled).await?;
        }
        if let Some(name) = &cmd.name {
            self.repo.update_name(channel.id, name).await?;
        }
        if let Some(status) = &cmd.status {
            self.repo.update_status(channel.id, Some(status)).await?;
        }
        if let Some(limit) = cmd.member_limit {
            self.repo.update_member_limit(channel.id, limit).await?;
        }
        if let Some(queue_ch) = &cmd.queue_channel_id {
            self.repo.update_queue_channel(channel.id, queue_ch.as_deref()).await?;
        }

        self.invalidate_cache(&channel.guild_id, &cmd.channel_id).await;
        Ok(())
    }

    async fn transfer_ownership(&self, cmd: TransferOwnershipCommand) -> Result<(), DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;
        self.repo.update_owner(channel.id, &cmd.new_owner_id, &cmd.new_owner_name).await?;
        self.invalidate_cache(&channel.guild_id, &cmd.channel_id).await;
        Ok(())
    }

    async fn add_co_admin(&self, cmd: ManageCoAdminCommand) -> Result<(), DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;

        let co_admin = VoiceChannelCoAdmin {
            id: Uuid::new_v4(),
            voice_channel_id: channel.id,
            user_id: cmd.user_id,
            user_name: cmd.user_name,
            granted_at: Utc::now(),
        };

        self.repo.add_co_admin(&co_admin).await?;
        self.invalidate_cache(&channel.guild_id, &cmd.channel_id).await;
        Ok(())
    }

    async fn remove_co_admin(&self, channel_id: &str, user_id: &str) -> Result<(), DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        self.repo.remove_co_admin(channel.id, user_id).await?;
        self.invalidate_cache(&channel.guild_id, channel_id).await;
        Ok(())
    }

    async fn get_whitelist(&self, guild_id: &str, owner_id: &str) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError> {
        self.repo.find_whitelist(guild_id, owner_id).await
    }

    async fn add_to_whitelist(&self, cmd: ManageWhitelistCommand) -> Result<(), DomainError> {
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

    async fn remove_from_whitelist(&self, guild_id: &str, owner_id: &str, target_id: &str) -> Result<(), DomainError> {
        self.repo.remove_from_whitelist(guild_id, owner_id, target_id).await
    }

    async fn ban_from_channel(&self, cmd: BanFromChannelCommand) -> Result<(), DomainError> {
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

    async fn unban_from_channel(&self, channel_id: &str, user_id: &str) -> Result<(), DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        self.repo.remove_ban(channel.id, user_id).await?;
        self.invalidate_cache(&channel.guild_id, channel_id).await;
        Ok(())
    }

    async fn is_banned(&self, channel_id: &str, user_id: &str) -> Result<bool, DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        let ban = self.repo.find_active_ban(channel.id, user_id).await?;
        Ok(ban.is_some())
    }
}
