//! gRPC Welcome config — delegue au WelcomeConfigRepository.
//! Plus de SQL direct : le repo centralise la query + defaults.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use sentinel_proto::welcome::v1 as proto;
use sentinel_proto::welcome::v1::welcome_service_server::WelcomeService;

use crate::ports::outbound::WelcomeConfigRepository;

pub struct WelcomeGrpc {
    pub repo: Arc<dyn WelcomeConfigRepository>,
}

#[tonic::async_trait]
impl WelcomeService for WelcomeGrpc {
    async fn get_config(
        &self,
        request: Request<proto::GetConfigRequest>,
    ) -> Result<Response<proto::WelcomeConfig>, Status> {
        let cfg = self.repo.get_config(&request.into_inner().guild_id).await
            .map_err(|e| Status::internal(format!("get welcome config: {e}")))?;

        Ok(Response::new(proto::WelcomeConfig {
            guild_id: cfg.guild_id,
            welcome_enabled: cfg.welcome_enabled,
            welcome_channel_id: cfg.welcome_channel_id,
            welcome_message: cfg.welcome_message,
            welcome_embed_color: cfg.welcome_embed_color,
            welcome_dm_enabled: cfg.welcome_dm_enabled,
            welcome_dm_message: cfg.welcome_dm_message,
            leave_enabled: cfg.leave_enabled,
            leave_channel_id: cfg.leave_channel_id,
            leave_message: cfg.leave_message,
            rules_enabled: cfg.rules_enabled,
            rules_channel_id: cfg.rules_channel_id,
            rules_message: cfg.rules_message,
            rules_role_id: cfg.rules_role_id,
            rules_button_label: cfg.rules_button_label,
            counter_enabled: cfg.counter_enabled,
            counter_channel_id: cfg.counter_channel_id,
            counter_format: cfg.counter_format,
            anniversary_enabled: cfg.anniversary_enabled,
            anniversary_channel_id: cfg.anniversary_channel_id,
            anniversary_message: cfg.anniversary_message,
            rejoin_message: cfg.rejoin_message,
        }))
    }
}
