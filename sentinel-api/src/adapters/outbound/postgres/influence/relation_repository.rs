//! Impl Postgres de `RelationRepository`.

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::influence::archive::{OrgRelation, RelationKind};
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::influence::relation_repository::RelationRepository;

use super::super::pg_err_ctx;

const TBL: &str = "influence_org_relations";

pub struct PgRelationRepository {
    pool: PgPool,
}

impl PgRelationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    other_org_name: String,
    relation: String,
}

#[async_trait]
impl RelationRepository for PgRelationRepository {
    async fn set(
        &self,
        guild_id: &str,
        org_id: Uuid,
        other_org_id: Uuid,
        relation: RelationKind,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO influence_org_relations (guild_id, org_id, other_org_id, relation) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (org_id, other_org_id) DO UPDATE SET relation = EXCLUDED.relation",
        )
        .bind(guild_id)
        .bind(org_id)
        .bind(other_org_id)
        .bind(relation.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| pg_err_ctx(TBL, e))?;
        Ok(())
    }

    async fn list_for_org(&self, org_id: Uuid) -> Result<Vec<OrgRelation>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT r.id AS id, o.name AS other_org_name, r.relation AS relation \
             FROM influence_org_relations r \
             JOIN influence_organizations o ON o.id = r.other_org_id \
             WHERE r.org_id = $1 ORDER BY r.created_at",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| pg_err_ctx(TBL, e))?;

        rows.into_iter()
            .map(|r| {
                let relation = RelationKind::from_str_lossy(&r.relation).ok_or_else(|| {
                    DomainError::Internal(format!("relation inconnue : {}", r.relation))
                })?;
                Ok(OrgRelation {
                    id: r.id,
                    other_org_name: r.other_org_name,
                    relation,
                })
            })
            .collect()
    }
}
