//! Implementation gRPC du `VoiceChannelsService` (Phase 7A).
//! Wrappe `ManageVoiceChannelsUseCase`. Invite-links non migres.

use std::sync::Arc;

use sentinel_proto::voice::v1 as proto;
use sentinel_proto::voice::v1::voice_channels_service_server::VoiceChannelsService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use sentinel_core::ports::inbound::community::manage_voice_channels::BanFromChannelCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::CreateVoiceChannelCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::ManageCoAdminCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::ManageVoiceChannelsUseCase;
use sentinel_core::ports::inbound::community::manage_voice_channels::ManageWhitelistCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::SavePresetCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::TransferOwnershipCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::UpdateVoiceChannelCommand;
use sentinel_core::domain::entities::community::voice_channel::VoiceChannel;
use sentinel_core::domain::errors::DomainError;
pub struct VoiceChannelsGrpc {
    pub uc: Arc<dyn ManageVoiceChannelsUseCase>,
}

#[tonic::async_trait]
impl VoiceChannelsService for VoiceChannelsGrpc {
    async fn list_channels(
        &self,
        request: Request<proto::ListChannelsRequest>,
    ) -> Result<Response<proto::VoiceChannelList>, Status> {
        let channels = self
            .uc
            .list_channels(&request.into_inner().guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::VoiceChannelList {
            channels: channels.into_iter().map(voice_channel_to_proto).collect(),
        }))
    }

    async fn create_channel(
        &self,
        request: Request<proto::CreateChannelRequest>,
    ) -> Result<Response<proto::VoiceChannel>, Status> {
        let req = request.into_inner();
        let channel = self
            .uc
            .create_channel(CreateVoiceChannelCommand {
                guild_id: req.guild_id.into(),
                owner_id: req.owner_id,
                owner_name: req.owner_name,
                channel_id: req.channel_id.into(),
                text_channel_id: req.text_channel_id,
                members_channel_id: req.members_channel_id,
                queue_channel_id: req.queue_channel_id,
                category_id: req.category_id,
                channel_name: req.channel_name,
                kind: req.kind,
                visibility: req.visibility,
                queue_enabled: req.queue_enabled,
                stage_enabled: false,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(voice_channel_to_proto(channel)))
    }

    async fn delete_channel(
        &self,
        request: Request<proto::DeleteChannelRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        self.uc
            .delete_channel(&request.into_inner().channel_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn update_channel(
        &self,
        request: Request<proto::UpdateChannelRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .update_channel(UpdateVoiceChannelCommand {
                channel_id: req.channel_id.into(),
                visibility: req.visibility,
                locked: req.locked,
                queue_enabled: req.queue_enabled,
                name: req.name,
                status: req.status,
                member_limit: req.member_limit.map(|w| w.value),
                queue_channel_id: req.queue_channel_id.map(|w| w.value),
                stage_enabled: None,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn get_channel(
        &self,
        request: Request<proto::GetChannelRequest>,
    ) -> Result<Response<proto::GetChannelResponse>, Status> {
        let req = request.into_inner();
        match self.uc.get_channel_detail(&req.channel_id).await {
            Ok(detail) => {
                let co_admins = detail
                    .co_admins
                    .iter()
                    .map(|ca| proto::CoAdmin {
                        user_id: ca.user_id.clone().into(),
                        user_name: ca.user_name.clone(),
                    })
                    .collect();
                Ok(Response::new(proto::GetChannelResponse {
                    channel: Some(voice_channel_to_proto(detail.channel)),
                    co_admins,
                }))
            }
            Err(DomainError::NotFound(_)) => Ok(Response::new(proto::GetChannelResponse {
                channel: None,
                co_admins: vec![],
            })),
            Err(e) => Err(domain_to_status(e)),
        }
    }

    async fn transfer_ownership(
        &self,
        request: Request<proto::TransferOwnershipRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .transfer_ownership(TransferOwnershipCommand {
                channel_id: req.channel_id.into(),
                new_owner_id: req.new_owner_id,
                new_owner_name: req.new_owner_name,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn add_co_admin(
        &self,
        request: Request<proto::AddCoAdminRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .add_co_admin(ManageCoAdminCommand {
                channel_id: req.channel_id.into(),
                user_id: req.user_id.into(),
                user_name: req.user_name,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn add_to_whitelist(
        &self,
        request: Request<proto::AddToWhitelistRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .add_to_whitelist(ManageWhitelistCommand {
                guild_id: req.guild_id.into(),
                owner_id: req.owner_id,
                target_id: req.target_id,
                target_name: req.target_name,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn ban_from_channel(
        &self,
        request: Request<proto::BanFromChannelRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .ban_from_channel(BanFromChannelCommand {
                channel_id: req.channel_id.into(),
                user_id: req.user_id.into(),
                user_name: req.user_name,
                banned_by: req.banned_by,
                reason: req.reason,
                duration_secs: req.duration_secs,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn is_banned(
        &self,
        request: Request<proto::IsBannedRequest>,
    ) -> Result<Response<proto::IsBannedResponse>, Status> {
        let req = request.into_inner();
        let banned = match self.uc.is_banned(&req.channel_id, &req.user_id).await {
            Ok(b) => b,
            // Salon inconnu cote DB : on ne bloque pas (pas de ban connu).
            Err(DomainError::NotFound(_)) => false,
            Err(e) => return Err(domain_to_status(e)),
        };
        Ok(Response::new(proto::IsBannedResponse { banned }))
    }

    async fn list_owner_bans(
        &self,
        request: Request<proto::ListOwnerBansRequest>,
    ) -> Result<Response<proto::OwnerBanList>, Status> {
        let req = request.into_inner();
        let bans = self
            .uc
            .list_owner_bans(&req.guild_id, &req.owner_id)
            .await
            .map_err(domain_to_status)?
            .into_iter()
            .map(|b| proto::OwnerBan {
                user_id: b.user_id.into(),
                user_name: b.user_name,
                reason: b.reason,
                expires_at: b.expires_at.map(|t| t.to_rfc3339()),
            })
            .collect();
        Ok(Response::new(proto::OwnerBanList { bans }))
    }

    async fn get_voice_config(
        &self,
        request: Request<proto::GetVoiceConfigRequest>,
    ) -> Result<Response<proto::VoiceConfig>, Status> {
        let cfg = self
            .uc
            .get_voice_config(&request.into_inner().guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::VoiceConfig {
            creation_cooldown_secs: cfg.creation_cooldown_secs,
            flood_max_messages: cfg.flood_max_messages,
            flood_time_window_secs: cfg.flood_time_window_secs,
            empty_cleanup_delay_secs: cfg.empty_cleanup_delay_secs,
            flood_mute_duration_secs: cfg.flood_mute_duration_secs,
            vote_kick_timeout_secs: cfg.vote_kick_timeout_secs,
        }))
    }

    async fn list_themes(
        &self,
        request: Request<proto::ListThemesRequest>,
    ) -> Result<Response<proto::ThemeList>, Status> {
        let themes = self
            .uc
            .list_themes(&request.into_inner().guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ThemeList {
            themes: themes.into_iter().map(voice_theme_to_proto).collect(),
        }))
    }

    async fn save_preset(
        &self,
        request: Request<proto::SavePresetRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .save_preset(SavePresetCommand {
                guild_id: req.guild_id.into(),
                owner_id: req.owner_id,
                channel_name: req.channel_name,
                member_limit: req.member_limit,
                visibility: req.visibility,
                locked: req.locked,
                queue_enabled: req.queue_enabled,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn get_preset(
        &self,
        request: Request<proto::GetPresetRequest>,
    ) -> Result<Response<proto::GetPresetResponse>, Status> {
        let req = request.into_inner();
        let preset = self
            .uc
            .get_preset(&req.guild_id, &req.owner_id)
            .await
            .map_err(domain_to_status)?
            .map(|p| proto::VoicePreset {
                owner_id: p.owner_id,
                channel_name: p.channel_name,
                member_limit: p.member_limit,
                visibility: p.visibility,
                locked: p.locked,
                queue_enabled: p.queue_enabled,
            });
        Ok(Response::new(proto::GetPresetResponse { preset }))
    }

    async fn get_whitelist(
        &self,
        request: Request<proto::GetWhitelistRequest>,
    ) -> Result<Response<proto::WhitelistList>, Status> {
        let req = request.into_inner();
        let entries = self
            .uc
            .get_whitelist(&req.guild_id, &req.owner_id)
            .await
            .map_err(domain_to_status)?
            .into_iter()
            .map(|e| proto::WhitelistEntry {
                target_id: e.target_id,
                target_name: e.target_name,
            })
            .collect();
        Ok(Response::new(proto::WhitelistList { entries }))
    }
}

fn voice_theme_to_proto(
    t: sentinel_core::domain::entities::community::voice_channel::VoiceChannelTheme,
) -> proto::VoiceChannelTheme {
    proto::VoiceChannelTheme {
        id: t.id.to_string(),
        guild_id: t.guild_id.into(),
        name: t.name,
        emoji: t.emoji,
        channel_name_template: t.channel_name_template,
        member_limit: t.member_limit,
        visibility: t.visibility,
        locked: t.locked,
        queue_enabled: t.queue_enabled,
        bitrate: t.bitrate,
        slowmode_secs: t.slowmode_secs,
        stage_enabled: t.stage_enabled,
        is_default: t.is_default,
        sort_order: t.sort_order,
        created_at: t.created_at.to_rfc3339(),
    }
}

fn voice_channel_to_proto(c: VoiceChannel) -> proto::VoiceChannel {
    proto::VoiceChannel {
        id: c.id.to_string(),
        guild_id: c.guild_id.into(),
        owner_id: c.owner_id,
        owner_name: c.owner_name,
        channel_id: c.channel_id.into(),
        text_channel_id: c.text_channel_id,
        members_channel_id: c.members_channel_id,
        queue_channel_id: c.queue_channel_id,
        category_id: c.category_id,
        channel_name: c.channel_name,
        kind: c.kind.as_str().to_string(),
        visibility: c.visibility,
        queue_enabled: c.queue_enabled,
        locked: c.locked,
        member_limit: c.member_limit,
        status: c.status,
        created_at: c.created_at.to_rfc3339(),
    }
}

#[cfg(test)]
#[path = "tests/voice.rs"]
mod tests;
