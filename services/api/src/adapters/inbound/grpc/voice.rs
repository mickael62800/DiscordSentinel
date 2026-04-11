//! Implementation gRPC du `VoiceChannelsService` (Phase 7A).
//! Wrappe `ManageVoiceChannelsUseCase`. Themes/invite-links non migres
//! (non utilises par api_client.rs du voice-bot).

use std::sync::Arc;

use tonic::{Request, Response, Status};

use sentinel_proto::voice::v1 as proto;
use sentinel_proto::voice::v1::voice_channels_service_server::VoiceChannelsService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::VoiceChannel;
use crate::domain::errors::DomainError;
use crate::ports::inbound::{
    BanFromChannelCommand, CreateVoiceChannelCommand, ManageCoAdminCommand,
    ManageVoiceChannelsUseCase, ManageWhitelistCommand, TransferOwnershipCommand,
    UpdateVoiceChannelCommand,
};

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
                channel_id: req.channel_id,
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
                channel_id: req.channel_id,
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
            Ok(detail) => Ok(Response::new(proto::GetChannelResponse {
                channel: Some(voice_channel_to_proto(detail.channel)),
            })),
            Err(DomainError::NotFound(_)) => Ok(Response::new(proto::GetChannelResponse {
                channel: None,
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
                channel_id: req.channel_id,
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
                channel_id: req.channel_id,
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
                channel_id: req.channel_id,
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
}

fn voice_channel_to_proto(c: VoiceChannel) -> proto::VoiceChannel {
    proto::VoiceChannel {
        id: c.id.to_string(),
        guild_id: c.guild_id,
        owner_id: c.owner_id,
        owner_name: c.owner_name,
        channel_id: c.channel_id,
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
