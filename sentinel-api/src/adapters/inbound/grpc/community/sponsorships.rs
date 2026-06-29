//! Phase 7A.opt F.3 — Community (sponsorships + temp-roles) gRPC.
//!
//! Copie du pattern du handler HTTP `bot_persistence.rs` : sqlx direct car
//! pas de use case unifie cote API. Permet au community-bot de faire tous
//! ses appels metier via gRPC (plus de HTTP fallback sur ce domaine).

use crate::adapters::inbound::grpc::errors::sqlx_to_status;
use chrono::DateTime;
use chrono::Utc;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use sentinel_proto::community::v1 as proto;
use sentinel_proto::community::v1::community_service_server::CommunityService;

pub struct CommunityGrpc {
    pub pg_pool: sqlx::PgPool,
}

#[derive(sqlx::FromRow)]
struct SponsorshipRow {
    id: sqlx::types::Uuid,
    guild_id: String,
    sponsor_id: String,
    sponsored_id: String,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct TempRoleRow {
    id: sqlx::types::Uuid,
    guild_id: String,
    user_id: String,
    role_id: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

#[tonic::async_trait]
impl CommunityService for CommunityGrpc {
    // ── Sponsorships ──

    async fn create_sponsorship(
        &self,
        request: Request<proto::CreateSponsorshipRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        sqlx::query(
            "INSERT INTO sponsorships (guild_id, sponsor_id, sponsored_id) \
             VALUES ($1, $2, $3) ON CONFLICT (guild_id, sponsored_id) DO NOTHING",
        )
        .bind(&req.guild_id)
        .bind(&req.sponsor_id)
        .bind(&req.sponsored_id)
        .execute(&self.pg_pool)
        .await
        .map_err(sqlx_to_status("INSERT sponsorship"))?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn list_sponsorships(
        &self,
        request: Request<proto::ListSponsorshipsRequest>,
    ) -> Result<Response<proto::SponsorshipList>, Status> {
        let req = request.into_inner();
        let rows = sqlx::query_as::<_, SponsorshipRow>(
            "SELECT id, guild_id, sponsor_id, sponsored_id, created_at \
             FROM sponsorships WHERE guild_id = $1 ORDER BY created_at DESC",
        )
        .bind(&req.guild_id)
        .fetch_all(&self.pg_pool)
        .await
        .map_err(sqlx_to_status("SELECT sponsorships"))?;
        Ok(Response::new(proto::SponsorshipList {
            sponsorships: rows
                .into_iter()
                .map(|r| proto::Sponsorship {
                    id: r.id.to_string(),
                    guild_id: r.guild_id,
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
        // Valider le format RFC3339 avant de passer a Postgres.
        chrono::DateTime::parse_from_rfc3339(&req.expires_at)
            .map_err(|_| Status::invalid_argument("expires_at doit etre au format RFC3339"))?;
        sqlx::query(
            "INSERT INTO temp_roles (guild_id, user_id, role_id, expires_at) \
             VALUES ($1, $2, $3, $4::timestamptz) \
             ON CONFLICT (guild_id, user_id, role_id) DO UPDATE SET expires_at = $4::timestamptz",
        )
        .bind(&req.guild_id)
        .bind(&req.user_id)
        .bind(&req.role_id)
        .bind(&req.expires_at)
        .execute(&self.pg_pool)
        .await
        .map_err(sqlx_to_status("INSERT temp_role"))?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn list_temp_roles(
        &self,
        request: Request<proto::ListTempRolesRequest>,
    ) -> Result<Response<proto::TempRoleList>, Status> {
        let req = request.into_inner();
        let rows = sqlx::query_as::<_, TempRoleRow>(
            "SELECT id, guild_id, user_id, role_id, expires_at, created_at \
             FROM temp_roles WHERE guild_id = $1 AND expires_at > NOW() \
             ORDER BY expires_at ASC",
        )
        .bind(&req.guild_id)
        .fetch_all(&self.pg_pool)
        .await
        .map_err(sqlx_to_status("SELECT temp_roles"))?;
        Ok(Response::new(proto::TempRoleList {
            roles: rows
                .into_iter()
                .map(|r| proto::TempRole {
                    id: r.id.to_string(),
                    guild_id: r.guild_id,
                    user_id: r.user_id,
                    role_id: r.role_id,
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
        sqlx::query("DELETE FROM temp_roles WHERE guild_id = $1 AND user_id = $2 AND role_id = $3")
            .bind(&req.guild_id)
            .bind(&req.user_id)
            .bind(&req.role_id)
            .execute(&self.pg_pool)
            .await
            .map_err(sqlx_to_status("DELETE temp_role"))?;
        Ok(Response::new(proto::Empty {}))
    }
}
