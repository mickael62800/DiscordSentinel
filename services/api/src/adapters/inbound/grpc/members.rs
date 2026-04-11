//! Implementation gRPC du `MembersService` (Phase 7A).
//! Wrappe `ManageMembersUseCase`. Partage par welcome-bot, security-bot, et
//! a terme par d'autres bots qui consomment le domaine membres.

use std::sync::Arc;

use chrono::DateTime;
use tonic::{Request, Response, Status};

use sentinel_proto::members::v1 as proto;
use sentinel_proto::members::v1::members_service_server::MembersService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::GuildMember;
use crate::domain::errors::DomainError;
use crate::ports::inbound::{
    ManageMembersUseCase, RegisterMemberCommand, SyncMembersCommand, UpdateMemberCommand,
};

pub struct MembersGrpc {
    pub uc: Arc<dyn ManageMembersUseCase>,
}

#[tonic::async_trait]
impl MembersService for MembersGrpc {
    async fn get_member(
        &self,
        request: Request<proto::GetMemberRequest>,
    ) -> Result<Response<proto::GetMemberResponse>, Status> {
        let req = request.into_inner();
        // Convertir NotFound en Option::None pour rester compatible avec
        // le contrat HTTP 404 que welcome-bot utilisait avec is_known_member.
        match self.uc.get_member(&req.guild_id, &req.user_id).await {
            Ok(m) => Ok(Response::new(proto::GetMemberResponse {
                member: Some(member_to_proto(m)?),
            })),
            Err(DomainError::NotFound(_)) => {
                Ok(Response::new(proto::GetMemberResponse { member: None }))
            }
            Err(e) => Err(domain_to_status(e)),
        }
    }

    async fn sync_members(
        &self,
        request: Request<proto::SyncMembersRequest>,
    ) -> Result<Response<proto::SyncMembersResponse>, Status> {
        let req = request.into_inner();
        let members = req
            .members
            .into_iter()
            .map(proto_to_member)
            .collect::<Result<Vec<_>, _>>()?;
        let count = self
            .uc
            .sync_members(SyncMembersCommand {
                guild_id: req.guild_id,
                members,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SyncMembersResponse {
            synced_count: count,
        }))
    }

    async fn register_member(
        &self,
        request: Request<proto::RegisterMemberRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let proto_member = req
            .member
            .ok_or_else(|| Status::invalid_argument("member manquant"))?;
        let member = proto_to_member(proto_member)?;
        self.uc
            .register_member(RegisterMemberCommand { member })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn remove_member(
        &self,
        request: Request<proto::RemoveMemberRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .remove_member(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn update_member(
        &self,
        request: Request<proto::UpdateMemberRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let roles = match req.roles_json.as_deref() {
            Some(s) => Some(
                serde_json::from_str(s)
                    .map_err(|e| Status::invalid_argument(format!("roles_json invalide: {e}")))?,
            ),
            None => None,
        };
        self.uc
            .update_member(UpdateMemberCommand {
                guild_id: req.guild_id,
                user_id: req.user_id,
                username: req.username,
                display_name: req.display_name,
                avatar: req.avatar,
                roles,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }
}

fn member_to_proto(m: GuildMember) -> Result<proto::GuildMember, Status> {
    let roles_json = serde_json::to_string(&m.roles)
        .map_err(|e| Status::internal(format!("serialisation roles: {e}")))?;
    Ok(proto::GuildMember {
        guild_id: m.guild_id,
        user_id: m.user_id,
        username: m.username,
        display_name: m.display_name,
        avatar: m.avatar,
        roles_json,
        joined_at: m.joined_at.map(|d| d.to_rfc3339()),
        account_created: m.account_created.map(|d| d.to_rfc3339()),
        is_bot: m.is_bot,
        last_seen_at: m.last_seen_at.map(|d| d.to_rfc3339()),
    })
}

fn proto_to_member(p: proto::GuildMember) -> Result<GuildMember, Status> {
    let roles = serde_json::from_str(&p.roles_json).unwrap_or(serde_json::Value::Array(vec![]));
    Ok(GuildMember {
        guild_id: p.guild_id,
        user_id: p.user_id,
        username: p.username,
        display_name: p.display_name,
        avatar: p.avatar,
        roles,
        joined_at: parse_rfc3339(p.joined_at)?,
        account_created: parse_rfc3339(p.account_created)?,
        is_bot: p.is_bot,
        last_seen_at: parse_rfc3339(p.last_seen_at)?,
    })
}

fn parse_rfc3339(s: Option<String>) -> Result<Option<DateTime<chrono::Utc>>, Status> {
    match s {
        None => Ok(None),
        Some(s) => DateTime::parse_from_rfc3339(&s)
            .map(|d| Some(d.with_timezone(&chrono::Utc)))
            .map_err(|e| Status::invalid_argument(format!("date invalide: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts() -> DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
    }

    fn sample_member() -> GuildMember {
        GuildMember {
            guild_id: "g1".into(),
            user_id: "u1".into(),
            username: "alice".into(),
            display_name: Some("Alice".into()),
            avatar: Some("hash123".into()),
            roles: serde_json::json!(["role1", "role2"]),
            joined_at: Some(ts()),
            account_created: Some(ts()),
            is_bot: false,
            last_seen_at: Some(ts()),
        }
    }

    #[test]
    fn member_to_proto_full_mapping() {
        let p = member_to_proto(sample_member()).unwrap();
        assert_eq!(p.guild_id, "g1");
        assert_eq!(p.user_id, "u1");
        assert_eq!(p.username, "alice");
        assert_eq!(p.display_name.as_deref(), Some("Alice"));
        assert!(p.roles_json.contains("role1"));
        assert_eq!(p.joined_at, Some(ts().to_rfc3339()));
        assert!(!p.is_bot);
    }

    #[test]
    fn member_to_proto_with_none_dates() {
        let mut m = sample_member();
        m.joined_at = None;
        m.account_created = None;
        m.last_seen_at = None;
        m.display_name = None;
        m.avatar = None;
        let p = member_to_proto(m).unwrap();
        assert!(p.joined_at.is_none());
        assert!(p.account_created.is_none());
        assert!(p.last_seen_at.is_none());
        assert!(p.display_name.is_none());
        assert!(p.avatar.is_none());
    }

    #[test]
    fn member_round_trip_via_proto() {
        let original = sample_member();
        let p = member_to_proto(original.clone()).unwrap();
        let back = proto_to_member(p).unwrap();
        assert_eq!(back.guild_id, original.guild_id);
        assert_eq!(back.user_id, original.user_id);
        assert_eq!(back.username, original.username);
        assert_eq!(back.display_name, original.display_name);
        assert_eq!(back.is_bot, original.is_bot);
        assert_eq!(back.joined_at, original.joined_at);
        assert_eq!(back.roles, original.roles);
    }

    #[test]
    fn proto_to_member_invalid_roles_json_falls_back_to_empty_array() {
        let p = proto::GuildMember {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "x".into(),
            display_name: None,
            avatar: None,
            roles_json: "not a json".into(),
            joined_at: None,
            account_created: None,
            is_bot: false,
            last_seen_at: None,
        };
        let m = proto_to_member(p).unwrap();
        assert_eq!(m.roles, serde_json::Value::Array(vec![]));
    }

    #[test]
    fn parse_rfc3339_none_yields_none() {
        assert_eq!(parse_rfc3339(None).unwrap(), None);
    }

    #[test]
    fn parse_rfc3339_valid_date() {
        let s = ts().to_rfc3339();
        let parsed = parse_rfc3339(Some(s)).unwrap();
        assert_eq!(parsed, Some(ts()));
    }

    #[test]
    fn parse_rfc3339_invalid_returns_invalid_argument() {
        let err = parse_rfc3339(Some("not-a-date".into())).unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        assert!(err.message().contains("date"));
    }
}
