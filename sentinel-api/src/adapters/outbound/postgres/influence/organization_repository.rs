//! Impl Postgres de `OrganizationRepository`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::influence::organization::Organization;
use sentinel_core::domain::enums::influence::organization_kind::OrganizationKind;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::influence::organization_repository::{
    NewOrganization, OrganizationRepository,
};

use super::super::pg_err_ctx;

const TBL: &str = "influence_organizations";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgOrganizationRepository {
    pool: PgPool,
}

impl PgOrganizationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: String,
    kind: String,
    name: String,
    motto: String,
    treasury: i64,
    reputation: i64,
    influence: i64,
    founder_id: Uuid,
    created_at: DateTime<Utc>,
    dissolved_at: Option<DateTime<Utc>>,
}

impl TryFrom<Row> for Organization {
    type Error = DomainError;
    fn try_from(r: Row) -> Result<Self, DomainError> {
        let kind = OrganizationKind::from_str_lossy(&r.kind)
            .ok_or_else(|| DomainError::Internal(format!("kind org inconnu : {}", r.kind)))?;
        Ok(Self {
            id: r.id,
            guild_id: r.guild_id,
            kind,
            name: r.name,
            motto: r.motto,
            treasury: r.treasury,
            reputation: r.reputation,
            influence: r.influence,
            founder_id: r.founder_id,
            created_at: r.created_at,
            dissolved_at: r.dissolved_at,
        })
    }
}

const SELECT_COLS: &str = "id, guild_id, kind, name, motto, treasury, reputation, \
    influence, founder_id, created_at, dissolved_at";

#[async_trait]
impl OrganizationRepository for PgOrganizationRepository {
    async fn create(&self, new: NewOrganization<'_>) -> Result<Organization, DomainError> {
        let sql = format!(
            "INSERT INTO influence_organizations (guild_id, kind, name, motto, founder_id) \
             VALUES ($1, $2, $3, $4, $5) RETURNING {SELECT_COLS}"
        );
        let row: Row = sqlx::query_as(&sql)
            .bind(new.guild_id)
            .bind(new.kind.as_str())
            .bind(new.name)
            .bind(new.motto)
            .bind(new.founder_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match &e {
                sqlx::Error::Database(db) if db.is_unique_violation() => {
                    DomainError::Conflict(format!("Nom d'organisation deja pris : {}", new.name))
                }
                _ => pg_err(e),
            })?;
        row.try_into()
    }

    async fn get(&self, id: Uuid) -> Result<Option<Organization>, DomainError> {
        let sql = format!("SELECT {SELECT_COLS} FROM influence_organizations WHERE id = $1");
        let row: Option<Row> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_name(
        &self,
        guild_id: &str,
        name: &str,
    ) -> Result<Option<Organization>, DomainError> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM influence_organizations \
             WHERE guild_id = $1 AND LOWER(name) = LOWER($2) AND dissolved_at IS NULL"
        );
        let row: Option<Row> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn count_active_founded_by(&self, founder_id: Uuid) -> Result<i64, DomainError> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM influence_organizations \
             WHERE founder_id = $1 AND dissolved_at IS NULL",
        )
        .bind(founder_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)
    }

    async fn list_for_guild(&self, guild_id: &str) -> Result<Vec<Organization>, DomainError> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM influence_organizations \
             WHERE guild_id = $1 AND dissolved_at IS NULL ORDER BY created_at"
        );
        let rows: Vec<Row> = sqlx::query_as(&sql)
            .bind(guild_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}
