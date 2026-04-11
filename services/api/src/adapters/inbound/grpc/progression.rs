//! Implementation gRPC du `ProgressionService`. Wrappe le use-case
//! `ManageLevelsUseCase` deja utilise par les handlers HTTP — meme logique
//! metier, meme broadcast d'event sur la WS.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use sentinel_proto::common::v1 as proto_common;
use sentinel_proto::progression::v1 as proto;
use sentinel_proto::progression::v1::progression_service_server::ProgressionService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::domain::entities::{xp_progress, LevelReward, UserLevel, XpSource};
use crate::ports::inbound::manage_levels::{AddXpCommand, AddXpResult, ManageLevelsUseCase};

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
                guild_id: req.guild_id,
                user_id: req.user_id,
                username: req.username,
                amount: req.amount,
                source,
            })
            .await
            .map_err(domain_to_status)?;

        // Meme broadcast WS que l'endpoint HTTP — les clients dashboard
        // continuent de recevoir les events live sans changement.
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
        let limit = if req.limit <= 0 { 25 } else { req.limit.min(100) };
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

    async fn get_rewards(
        &self,
        request: Request<proto::GetRewardsRequest>,
    ) -> Result<Response<proto::RewardList>, Status> {
        let req = request.into_inner();
        let rewards = self
            .levels_uc
            .get_rewards(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::RewardList {
            rewards: rewards.into_iter().map(level_reward_to_proto).collect(),
        }))
    }
}

// ── Conversions domain <-> proto ──

fn xp_source_from_proto(value: i32) -> XpSource {
    match proto_common::XpSource::try_from(value).unwrap_or(proto_common::XpSource::Unspecified) {
        proto_common::XpSource::Voice => XpSource::Voice,
        // UNSPECIFIED ou TEXT -> Text (default cohérent avec l'API HTTP).
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
        // Days n'est pas exposé en proto v1 — fallback Text pour rester compatible.
        XpSource::Days => proto_common::XpSource::Text as i32,
    }
}

fn user_level_to_proto(u: UserLevel) -> proto::UserLevel {
    let (xp_current, xp_needed) = xp_progress(u.xp);
    let (xp_text_current, xp_text_needed) = xp_progress(u.xp_text);
    let (xp_voice_current, xp_voice_needed) = xp_progress(u.xp_voice);
    proto::UserLevel {
        id: u.id.to_string(),
        guild_id: u.guild_id,
        user_id: u.user_id,
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

fn level_reward_to_proto(r: LevelReward) -> proto::LevelReward {
    proto::LevelReward {
        id: r.id.to_string(),
        guild_id: r.guild_id,
        level: r.level,
        role_id: r.role_id,
        source: xp_source_to_proto(r.source),
    }
}

fn add_xp_result_to_proto(r: AddXpResult) -> proto::AddXpResponse {
    proto::AddXpResponse {
        user: Some(user_level_to_proto(r.user_level)),
        leveled_up: r.leveled_up,
        old_level: r.old_level,
        reward_role_id: r.reward_role_id,
        source: xp_source_to_proto(r.source),
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

    fn sample_user_level() -> UserLevel {
        UserLevel {
            id: Uuid::nil(),
            guild_id: "g1".into(),
            user_id: "u1".into(),
            username: "alice".into(),
            xp: 500,
            level: 5,
            xp_text: 300,
            level_text: 3,
            xp_voice: 200,
            level_voice: 2,
            last_xp_at: ts(),
            created_at: ts(),
            updated_at: ts(),
        }
    }

    #[test]
    fn xp_source_from_proto_voice_maps_correctly() {
        assert_eq!(
            xp_source_from_proto(proto_common::XpSource::Voice as i32),
            XpSource::Voice
        );
    }

    #[test]
    fn xp_source_from_proto_text_maps_correctly() {
        assert_eq!(
            xp_source_from_proto(proto_common::XpSource::Text as i32),
            XpSource::Text
        );
    }

    #[test]
    fn xp_source_from_proto_unspecified_defaults_to_text() {
        assert_eq!(
            xp_source_from_proto(proto_common::XpSource::Unspecified as i32),
            XpSource::Text
        );
        // Valeur invalide -> Text aussi (fallback safe).
        assert_eq!(xp_source_from_proto(9999), XpSource::Text);
    }

    #[test]
    fn xp_source_opt_from_proto_distinguishes_unspecified() {
        assert_eq!(
            xp_source_opt_from_proto(proto_common::XpSource::Text as i32),
            Some(XpSource::Text)
        );
        assert_eq!(
            xp_source_opt_from_proto(proto_common::XpSource::Voice as i32),
            Some(XpSource::Voice)
        );
        assert_eq!(
            xp_source_opt_from_proto(proto_common::XpSource::Unspecified as i32),
            None,
            "Unspecified doit retourner None pour distinguer 'aucun filtre'"
        );
    }

    #[test]
    fn xp_source_to_proto_round_trip_text_voice() {
        assert_eq!(
            xp_source_to_proto(XpSource::Text),
            proto_common::XpSource::Text as i32
        );
        assert_eq!(
            xp_source_to_proto(XpSource::Voice),
            proto_common::XpSource::Voice as i32
        );
    }

    #[test]
    fn xp_source_to_proto_days_falls_back_to_text() {
        // Days n'existe pas en proto v1 — fallback Text pour compat.
        assert_eq!(
            xp_source_to_proto(XpSource::Days),
            proto_common::XpSource::Text as i32
        );
    }

    #[test]
    fn user_level_to_proto_full_mapping() {
        let u = sample_user_level();
        let p = user_level_to_proto(u);
        assert_eq!(p.guild_id, "g1");
        assert_eq!(p.user_id, "u1");
        assert_eq!(p.username, "alice");
        assert_eq!(p.xp, 500);
        assert_eq!(p.level, 5);
        assert_eq!(p.xp_text, 300);
        assert_eq!(p.level_text, 3);
        assert_eq!(p.xp_voice, 200);
        assert_eq!(p.level_voice, 2);
        assert_eq!(p.last_xp_at, ts().to_rfc3339());
        // xp_progress doit calculer xp_current/xp_needed coherents.
        assert!(p.xp_needed > 0);
    }

    #[test]
    fn level_reward_to_proto_full_mapping() {
        let r = LevelReward {
            id: Uuid::nil(),
            guild_id: "g".into(),
            level: 10,
            role_id: "role42".into(),
            source: XpSource::Voice,
        };
        let p = level_reward_to_proto(r);
        assert_eq!(p.id, Uuid::nil().to_string());
        assert_eq!(p.level, 10);
        assert_eq!(p.role_id, "role42");
        assert_eq!(p.source, proto_common::XpSource::Voice as i32);
    }

    #[test]
    fn add_xp_result_to_proto_levelup_with_reward() {
        let r = AddXpResult {
            user_level: sample_user_level(),
            leveled_up: true,
            old_level: 4,
            reward_role_id: Some("reward_role".into()),
            source: XpSource::Text,
        };
        let p = add_xp_result_to_proto(r);
        assert!(p.leveled_up);
        assert_eq!(p.old_level, 4);
        assert_eq!(p.reward_role_id.as_deref(), Some("reward_role"));
        assert_eq!(p.source, proto_common::XpSource::Text as i32);
        assert!(p.user.is_some());
        assert_eq!(p.user.unwrap().level, 5);
    }

    #[test]
    fn add_xp_result_to_proto_no_levelup_no_reward() {
        let r = AddXpResult {
            user_level: sample_user_level(),
            leveled_up: false,
            old_level: 5,
            reward_role_id: None,
            source: XpSource::Voice,
        };
        let p = add_xp_result_to_proto(r);
        assert!(!p.leveled_up);
        assert!(p.reward_role_id.is_none());
    }
}
