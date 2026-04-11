//! Implementation gRPC du `StatsService`. Wrappe le use-case
//! `ManageStatsUseCase` deja utilise par les handlers HTTP — meme broadcast WS.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use sentinel_proto::stats::v1 as proto;
use sentinel_proto::stats::v1::stats_service_server::StatsService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::domain::entities::{GuildStatsOverview, UserStats};
use crate::ports::inbound::manage_stats::{
    ManageStatsUseCase, RecordMessagesCommand, RecordVoiceCommand,
};

pub struct StatsGrpc {
    pub stats_uc: Arc<dyn ManageStatsUseCase>,
    pub broadcaster: Arc<EventBroadcaster>,
}

#[tonic::async_trait]
impl StatsService for StatsGrpc {
    async fn record_messages(
        &self,
        request: Request<proto::RecordMessagesRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let guild_id = req.guild_id.clone();
        let user_id = req.user_id.clone();
        let count = req.count;

        self.stats_uc
            .record_messages(RecordMessagesCommand {
                guild_id: req.guild_id,
                user_id: req.user_id,
                username: req.username,
                count: req.count,
            })
            .await
            .map_err(domain_to_status)?;

        self.broadcaster.broadcast(
            "stats_messages_recorded",
            serde_json::json!({ "guild_id": &guild_id, "user_id": &user_id, "count": count }),
        );
        Ok(Response::new(proto::Empty {}))
    }

    async fn record_voice(
        &self,
        request: Request<proto::RecordVoiceRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let guild_id = req.guild_id.clone();
        let user_id = req.user_id.clone();
        let seconds = req.seconds;

        self.stats_uc
            .record_voice(RecordVoiceCommand {
                guild_id: req.guild_id,
                user_id: req.user_id,
                username: req.username,
                seconds: req.seconds,
                channel_id: req.channel_id,
                channel_name: req.channel_name,
            })
            .await
            .map_err(domain_to_status)?;

        self.broadcaster.broadcast(
            "stats_voice_recorded",
            serde_json::json!({ "guild_id": &guild_id, "user_id": &user_id, "seconds": seconds }),
        );
        Ok(Response::new(proto::Empty {}))
    }

    async fn get_user_stats(
        &self,
        request: Request<proto::GetUserStatsRequest>,
    ) -> Result<Response<proto::GetUserStatsResponse>, Status> {
        let req = request.into_inner();
        let stats = self
            .stats_uc
            .get_user_stats(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::GetUserStatsResponse {
            stats: stats.map(user_stats_to_proto),
        }))
    }

    async fn get_guild_overview(
        &self,
        request: Request<proto::GetGuildOverviewRequest>,
    ) -> Result<Response<proto::GuildOverview>, Status> {
        let req = request.into_inner();
        let overview = self
            .stats_uc
            .get_guild_overview(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(guild_overview_to_proto(overview)))
    }

    async fn get_leaderboard(
        &self,
        request: Request<proto::GetLeaderboardRequest>,
    ) -> Result<Response<proto::UserStatsList>, Status> {
        let req = request.into_inner();
        let limit = if req.limit == 0 { 10 } else { req.limit.min(50) };
        let users = self
            .stats_uc
            .get_leaderboard(&req.guild_id, limit)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::UserStatsList {
            users: users.into_iter().map(user_stats_to_proto).collect(),
        }))
    }
}

fn user_stats_to_proto(u: UserStats) -> proto::UserStats {
    proto::UserStats {
        id: u.id.to_string(),
        guild_id: u.guild_id,
        user_id: u.user_id,
        username: u.username,
        message_count: u.message_count,
        voice_seconds: u.voice_seconds,
        updated_at: u.updated_at.to_rfc3339(),
    }
}

fn guild_overview_to_proto(o: GuildStatsOverview) -> proto::GuildOverview {
    proto::GuildOverview {
        guild_id: o.guild_id,
        total_messages: o.total_messages,
        total_voice_seconds: o.total_voice_seconds,
        active_members: o.active_members,
        total_infractions: o.total_infractions,
        total_warns: o.total_warns,
        total_mutes: o.total_mutes,
        total_bans: o.total_bans,
        top_members: o.top_members.into_iter().map(user_stats_to_proto).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    fn sample_user_stats() -> UserStats {
        UserStats {
            id: Uuid::nil(),
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "alice".into(),
            message_count: 1500,
            voice_seconds: 7200,
            updated_at: ts(),
        }
    }

    #[test]
    fn user_stats_to_proto_full_mapping() {
        let p = user_stats_to_proto(sample_user_stats());
        assert_eq!(p.user_id, "u");
        assert_eq!(p.message_count, 1500);
        assert_eq!(p.voice_seconds, 7200);
        assert_eq!(p.updated_at, ts().to_rfc3339());
    }

    #[test]
    fn guild_overview_to_proto_full_mapping() {
        let o = GuildStatsOverview {
            guild_id: "g1".into(),
            total_messages: 50000,
            total_voice_seconds: 360000,
            active_members: 200,
            total_infractions: 30,
            total_warns: 20,
            total_mutes: 8,
            total_bans: 2,
            top_members: vec![sample_user_stats(), sample_user_stats(), sample_user_stats()],
        };
        let p = guild_overview_to_proto(o);
        assert_eq!(p.guild_id, "g1");
        assert_eq!(p.total_messages, 50000);
        assert_eq!(p.total_voice_seconds, 360000);
        assert_eq!(p.active_members, 200);
        assert_eq!(p.total_warns + p.total_mutes + p.total_bans, 30);
        assert_eq!(p.top_members.len(), 3);
    }

    #[test]
    fn guild_overview_to_proto_empty_top_members() {
        let o = GuildStatsOverview {
            guild_id: "g".into(),
            total_messages: 0, total_voice_seconds: 0, active_members: 0,
            total_infractions: 0, total_warns: 0, total_mutes: 0, total_bans: 0,
            top_members: vec![],
        };
        let p = guild_overview_to_proto(o);
        assert!(p.top_members.is_empty());
    }
}
