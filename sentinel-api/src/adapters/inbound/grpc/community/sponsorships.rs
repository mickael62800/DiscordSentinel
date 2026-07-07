//! Phase 7A.opt F.3 — Community (sponsorships + temp-roles) gRPC.
//!
//! Adaptateur inbound MINCE : parse/mappe le proto et delegue toute la logique
//! metier + persistance au use case `ManageSponsorshipsUseCase` (aucun sqlx ni
//! pg_pool ici).

use std::sync::Arc;

use tonic::Request;
use tonic::Response;
use tonic::Status;

use sentinel_core::ports::inbound::community::manage_sponsorships::ManageSponsorshipsUseCase;
use sentinel_proto::community::v1 as proto;
use sentinel_proto::community::v1::community_service_server::CommunityService;

use crate::adapters::inbound::grpc::errors::domain_to_status;

pub struct CommunityGrpc {
    pub uc: Arc<dyn ManageSponsorshipsUseCase>,
}

#[tonic::async_trait]
impl CommunityService for CommunityGrpc {
    // ── Sponsorships ──

    async fn create_sponsorship(
        &self,
        request: Request<proto::CreateSponsorshipRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .create_sponsorship(&req.guild_id, &req.sponsor_id, &req.sponsored_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn list_sponsorships(
        &self,
        request: Request<proto::ListSponsorshipsRequest>,
    ) -> Result<Response<proto::SponsorshipList>, Status> {
        let req = request.into_inner();
        let rows = self
            .uc
            .list_sponsorships(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SponsorshipList {
            sponsorships: rows
                .into_iter()
                .map(|r| proto::Sponsorship {
                    id: r.id.to_string(),
                    guild_id: r.guild_id.into_inner(),
                    sponsor_id: r.sponsor_id,
                    sponsored_id: r.sponsored_id,
                    created_at: r.created_at.to_rfc3339(),
                })
                .collect(),
        }))
    }

    // ── Temp Roles ──

    async fn create_temp_role(
        &self,
        request: Request<proto::CreateTempRoleRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .create_temp_role(&req.guild_id, &req.user_id, &req.role_id, &req.expires_at)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn list_temp_roles(
        &self,
        request: Request<proto::ListTempRolesRequest>,
    ) -> Result<Response<proto::TempRoleList>, Status> {
        let req = request.into_inner();
        let rows = self
            .uc
            .list_temp_roles(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::TempRoleList {
            roles: rows
                .into_iter()
                .map(|r| proto::TempRole {
                    id: r.id.to_string(),
                    guild_id: r.guild_id.into_inner(),
                    user_id: r.user_id.into_inner(),
                    role_id: r.role_id.into_inner(),
                    expires_at: r.expires_at.to_rfc3339(),
                    created_at: r.created_at.to_rfc3339(),
                })
                .collect(),
        }))
    }

    async fn delete_temp_role(
        &self,
        request: Request<proto::DeleteTempRoleRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.uc
            .delete_temp_role(&req.guild_id, &req.user_id, &req.role_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }
}
