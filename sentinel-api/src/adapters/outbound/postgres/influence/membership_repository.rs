//! Impl Postgres de `MembershipRepository`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::influence::org_membership::{
    OrgMember, OrgMemberView, OrgRole,
};
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::influence::membership_repository::MembershipRepository;

use super::super::pg_err_ctx;

const TBL: &str = "influence_org_members";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgMembershipRepository {
    pool: PgPool,
}

impl PgMembershipRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct MemberRow {
    id: Uuid,
    org_id: Uuid,
    citizen_id: Uuid,
    role: String,
    joined_at: DateTime<Utc>,
}

impl TryFrom<MemberRow> for OrgMember {
    type Error = DomainError;
    fn try_from(r: MemberRow) -> Result<Self, DomainError> {
        let role = OrgRole::from_str_lossy(&r.role)
            .ok_or_else(|| DomainError::Internal(format!("role inconnu : {}", r.role)))?;
        Ok(Self {
            id: r.id,
            org_id: r.org_id,
            citizen_id: r.citizen_id,
            role,
            joined_at: r.joined_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ViewRow {
    username: String,
    role: String,
    joined_at: DateTime<Utc>,
}

#[async_trait]
impl MembershipRepository for PgMembershipRepository {
    async fn add(
        &self,
        org_id: Uuid,
        citizen_id: Uuid,
        role: OrgRole,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO influence_org_members (org_id, citizen_id, role) VALUES ($1, $2, $3)",
        )
        .bind(org_id)
        .bind(citizen_id)
        .bind(role.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                DomainError::Conflict("Deja membre de cette organisation.".to_string())
            }
            _ => pg_err(e),
        })?;
        Ok(())
    }

    async fn get(
        &self,
        org_id: Uuid,
        citizen_id: Uuid,
    ) -> Result<Option<OrgMember>, DomainError> {
        let row: Option<MemberRow> = sqlx::query_as(
            "SELECT id, org_id, citizen_id, role, joined_at FROM influence_org_members \
             WHERE org_id = $1 AND citizen_id = $2",
        )
        .bind(org_id)
        .bind(citizen_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list_views(&self, org_id: Uuid) -> Result<Vec<OrgMemberView>, DomainError> {
        // Tri par rang hierarchique (Fondateur d'abord) puis anciennete.
        let rows: Vec<ViewRow> = sqlx::query_as(
            "SELECT c.username AS username, m.role AS role, m.joined_at AS joined_at \
             FROM influence_org_members m \
             JOIN influence_citizens c ON c.id = m.citizen_id \
             WHERE m.org_id = $1 \
             ORDER BY CASE m.role \
                 WHEN 'fondateur' THEN 0 WHEN 'dirigeant' THEN 1 \
                 WHEN 'responsable' THEN 2 WHEN 'membre' THEN 3 ELSE 4 END, \
                 m.joined_at",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        rows.into_iter()
            .map(|r| {
                let role = OrgRole::from_str_lossy(&r.role)
                    .ok_or_else(|| DomainError::Internal(format!("role inconnu : {}", r.role)))?;
                Ok(OrgMemberView {
                    username: r.username,
                    role,
                    joined_at: r.joined_at,
                })
            })
            .collect()
    }

    async fn count(&self, org_id: Uuid) -> Result<i64, DomainError> {
        sqlx::query_scalar("SELECT COUNT(*) FROM influence_org_members WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(&self.pool)
            .await
            .map_err(pg_err)
    }
}
