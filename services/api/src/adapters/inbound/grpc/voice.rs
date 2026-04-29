//! Implementation gRPC du `VoiceChannelsService` (Phase 7A).
//! Wrappe `ManageVoiceChannelsUseCase`. Invite-links non migres.

use std::sync::Arc;

use tonic::Request;
use tonic::Response;
use tonic::Status;
use sentinel_proto::voice::v1 as proto;
use sentinel_proto::voice::v1::voice_channels_service_server::VoiceChannelsService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::community::voice_channel::VoiceChannel;
use crate::domain::errors::DomainError;
use crate::ports::inbound::community::manage_voice_channels::BanFromChannelCommand;
use crate::ports::inbound::community::manage_voice_channels::CreateVoiceChannelCommand;
use crate::ports::inbound::community::manage_voice_channels::ManageCoAdminCommand;
use crate::ports::inbound::community::manage_voice_channels::ManageVoiceChannelsUseCase;
use crate::ports::inbound::community::manage_voice_channels::ManageWhitelistCommand;
use crate::ports::inbound::community::manage_voice_channels::TransferOwnershipCommand;
use crate::ports::inbound::community::manage_voice_channels::UpdateVoiceChannelCommand;
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
                guild_id: req.guild_id,
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
                        user_id: ca.user_id.clone(),
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
                user_id: req.user_id,
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
                guild_id: req.guild_id,
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
                user_id: req.user_id,
                user_name: req.user_name,
                banned_by: req.banned_by,
                reason: req.reason,
                duration_secs: req.duration_secs,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
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
}

fn voice_theme_to_proto(t: crate::domain::entities::community::voice_channel::VoiceChannelTheme) -> proto::VoiceChannelTheme {
    proto::VoiceChannelTheme {
        id: t.id.to_string(),
        guild_id: t.guild_id,
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
        guild_id: c.guild_id,
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
