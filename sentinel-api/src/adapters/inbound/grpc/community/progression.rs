//! Implementation gRPC du `ProgressionService`. Wrappe le use-case
//! `ManageLevelsUseCase` deja utilise par les handlers HTTP — meme logique
//! metier, meme broadcast d'event sur la WS.

use std::sync::Arc;

use sentinel_proto::common::v1 as proto_common;
use sentinel_proto::progression::v1 as proto;
use sentinel_proto::progression::v1::progression_service_server::ProgressionService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::ports::inbound::community::manage_levels::AddXpCommand;
use crate::ports::inbound::community::manage_levels::AddXpResult;
use crate::ports::inbound::community::manage_levels::ManageLevelsUseCase;
use sentinel_core::domain::entities::community::level::xp_progress;
use sentinel_core::domain::entities::community::level::UserLevel;
use sentinel_core::domain::entities::community::level::XpSource;
pub struct ProgressionGrpc {
    pub levels_uc: Arc<dyn ManageLevelsUseCase>,
    pub broadcaster: Arc<EventBroadcaster>,
}

#[tonic::async_trait]
impl ProgressionService for ProgressionGrpc {
    async fn add_xp(
        &self,
        request: Request<proto::AddXpRequest>,
    ) -> Result<Response<proto::AddXpResponse>, Status> {
        let req = request.into_inner();
        let source = xp_source_from_proto(req.source);
        let guild_id = req.guild_id.clone();
        let user_id = req.user_id.clone();
        let amount = req.amount;

        let result = self
            .levels_uc
            .add_xp(AddXpCommand {
                guild_id: req.guild_id.into(),
                user_id: req.user_id.into(),
                username: req.username,
                amount: req.amount,
                source,
            })
            .await
            .map_err(domain_to_status)?;

        self.broadcaster.broadcast(
            "xp_gained",
            serde_json::json!({
                "guild_id": &guild_id,
                "user_id": &user_id,
                "amount": amount,
                "source": source.as_str(),
            }),
        );

        Ok(Response::new(add_xp_result_to_proto(result)))
    }

    async fn get_user_level(
        &self,
        request: Request<proto::GetUserLevelRequest>,
    ) -> Result<Response<proto::UserLevel>, Status> {
        let req = request.into_inner();
        let level = self
            .levels_uc
            .get_user_level(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(user_level_to_proto(level)))
    }

    async fn get_leaderboard(
        &self,
        request: Request<proto::GetLeaderboardRequest>,
    ) -> Result<Response<proto::Leaderboard>, Status> {
        let req = request.into_inner();
        let limit = if req.limit <= 0 {
            25
        } else {
            req.limit.min(100)
        };
        let users = match xp_source_opt_from_proto(req.source) {
            Some(src) => self
                .levels_uc
                .get_leaderboard_by_source(&req.guild_id, src, limit)
                .await
                .map_err(domain_to_status)?,
            None => self
                .levels_uc
                .get_leaderboard(&req.guild_id, limit)
                .await
                .map_err(domain_to_status)?,
        };
        Ok(Response::new(proto::Leaderboard {
            users: users.into_iter().map(user_level_to_proto).collect(),
        }))
    }
}

// ── Conversions domain <-> proto ──

fn xp_source_from_proto(value: i32) -> XpSource {
    match proto_common::XpSource::try_from(value).unwrap_or(proto_common::XpSource::Unspecified) {
        proto_common::XpSource::Voice => XpSource::Voice,
        // UNSPECIFIED ou TEXT -> Text (default coherent avec l'API HTTP).
        _ => XpSource::Text,
    }
}

fn xp_source_opt_from_proto(value: i32) -> Option<XpSource> {
    match proto_common::XpSource::try_from(value).ok()? {
        proto_common::XpSource::Text => Some(XpSource::Text),
        proto_common::XpSource::Voice => Some(XpSource::Voice),
        proto_common::XpSource::Unspecified => None,
    }
}

fn xp_source_to_proto(source: XpSource) -> i32 {
    match source {
        XpSource::Text => proto_common::XpSource::Text as i32,
        XpSource::Voice => proto_common::XpSource::Voice as i32,
    }
}

fn user_level_to_proto(u: UserLevel) -> proto::UserLevel {
    let (xp_current, xp_needed) = xp_progress(u.xp);
    let (xp_text_current, xp_text_needed) = xp_progress(u.xp_text);
    let (xp_voice_current, xp_voice_needed) = xp_progress(u.xp_voice);
    proto::UserLevel {
        id: u.id.to_string(),
        guild_id: u.guild_id.into(),
        user_id: u.user_id.into(),
        username: u.username,
        xp: u.xp,
        level: u.level,
        xp_current,
        xp_needed,
        xp_text: u.xp_text,
        level_text: u.level_text,
        xp_text_current,
        xp_text_needed,
        xp_voice: u.xp_voice,
        level_voice: u.level_voice,
        xp_voice_current,
        xp_voice_needed,
        last_xp_at: u.last_xp_at.to_rfc3339(),
    }
}

fn add_xp_result_to_proto(r: AddXpResult) -> proto::AddXpResponse {
    proto::AddXpResponse {
        user: Some(user_level_to_proto(r.user_level)),
        leveled_up: r.leveled_up,
        old_level: r.old_level,
        old_level_global: r.old_level_global,
        source: xp_source_to_proto(r.source),
    }
}

#[cfg(test)]
#[path = "tests/progression.rs"]
mod tests;
