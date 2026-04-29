//! Implementation gRPC du `MembersService` (Phase 7A).
//! Wrappe `ManageMembersUseCase`. Partage par welcome-bot, security-bot, et
//! a terme par d'autres bots qui consomment le domaine membres.

use std::sync::Arc;

use chrono::DateTime;
use tonic::Request;
use tonic::Response;
use tonic::Status;
use sentinel_proto::members::v1 as proto;
use sentinel_proto::members::v1::members_service_server::MembersService;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::domain::entities::community::guild_member::GuildMember;
use crate::domain::errors::DomainError;
use crate::ports::inbound::community::manage_members::ManageMembersUseCase;
use crate::ports::inbound::community::manage_members::RegisterMemberCommand;
use crate::ports::inbound::community::manage_members::SyncMembersCommand;
use crate::ports::inbound::community::manage_members::UpdateMemberCommand;
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
                guild_id: req.guild_id.into(),
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
                guild_id: req.guild_id.into(),
                user_id: req.user_id.into(),
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
        guild_id: m.guild_id.into(),
        user_id: m.user_id.into(),
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
        guild_id: p.guild_id.into(),
        user_id: p.user_id.into(),
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
#[path = "tests/members.rs"]
mod tests;
