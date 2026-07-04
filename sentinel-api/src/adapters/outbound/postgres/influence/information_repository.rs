//! Impl Postgres : InvestigationRepository, InformationRepository, ArchiveRepository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::influence::information::{
    Information, Investigation, InvestigationStatus, Visibility,
};
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::influence::information_repository::{
    ArchiveRepository, InformationRepository, InvestigationRepository, NewInformation,
    NewInvestigation,
};

use super::super::pg_err_ctx;

// ── Investigations ──

pub struct PgInvestigationRepository {
    pool: PgPool,
}
impl PgInvestigationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct InvRow {
    id: Uuid,
    guild_id: String,
    initiator_id: Uuid,
    initiator_user_id: String,
    target_user_id: String,
    target_username: String,
    subject: String,
    status: String,
    resolves_at: DateTime<Utc>,
}

impl TryFrom<InvRow> for Investigation {
    type Error = DomainError;
    fn try_from(r: InvRow) -> Result<Self, DomainError> {
        let status = InvestigationStatus::from_str_lossy(&r.status)
            .ok_or_else(|| DomainError::Internal(format!("statut enquete inconnu : {}", r.status)))?;
        Ok(Self {
            id: r.id,
            guild_id: r.guild_id,
            initiator_id: r.initiator_id,
            initiator_user_id: r.initiator_user_id,
            target_user_id: r.target_user_id,
            target_username: r.target_username,
            subject: r.subject,
            status,
            resolves_at: r.resolves_at,
        })
    }
}

const INV_COLS: &str = "id, guild_id, initiator_id, initiator_user_id, target_user_id, \
    target_username, subject, status, resolves_at";

#[async_trait]
impl InvestigationRepository for PgInvestigationRepository {
    async fn create(&self, new: NewInvestigation<'_>) -> Result<Investigation, DomainError> {
        let sql = format!(
            "INSERT INTO influence_investigations \
             (guild_id, initiator_id, initiator_user_id, target_user_id, target_username, subject, resolves_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {INV_COLS}"
        );
        let row: InvRow = sqlx::query_as(&sql)
            .bind(new.guild_id)
            .bind(new.initiator_id)
            .bind(new.initiator_user_id)
            .bind(new.target_user_id)
            .bind(new.target_username)
            .bind(new.subject)
            .bind(new.resolves_at)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| pg_err_ctx("influence_investigations", e))?;
        row.try_into()
    }

    async fn list_due(&self, now: DateTime<Utc>) -> Result<Vec<Investigation>, DomainError> {
        let sql = format!(
            "SELECT {INV_COLS} FROM influence_investigations \
             WHERE status = 'en_cours' AND resolves_at <= $1"
        );
        let rows: Vec<InvRow> = sqlx::query_as(&sql)
            .bind(now)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| pg_err_ctx("influence_investigations", e))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn resolve(
        &self,
        id: Uuid,
        status: InvestigationStatus,
        info_id: Option<Uuid>,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE influence_investigations SET status = $2, info_id = $3 WHERE id = $1")
            .bind(id)
            .bind(status.as_str())
            .bind(info_id)
            .execute(&self.pool)
            .await
            .map_err(|e| pg_err_ctx("influence_investigations", e))?;
        Ok(())
    }
}

// ── Informations ──

pub struct PgInformationRepository {
    pool: PgPool,
}
impl PgInformationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct InfoRow {
    id: Uuid,
    guild_id: String,
    owner_id: Uuid,
    target_user_id: String,
    target_username: String,
    content: String,
    visibility: String,
    revealed: bool,
    created_at: DateTime<Utc>,
}

impl TryFrom<InfoRow> for Information {
    type Error = DomainError;
    fn try_from(r: InfoRow) -> Result<Self, DomainError> {
        let visibility = Visibility::from_str_lossy(&r.visibility)
            .ok_or_else(|| DomainError::Internal(format!("visibilite inconnue : {}", r.visibility)))?;
        Ok(Self {
            id: r.id,
            guild_id: r.guild_id,
            owner_id: r.owner_id,
            target_user_id: r.target_user_id,
            target_username: r.target_username,
            content: r.content,
            visibility,
            revealed: r.revealed,
            created_at: r.created_at,
        })
    }
}

const INFO_COLS: &str = "id, guild_id, owner_id, target_user_id, target_username, content, \
    visibility, revealed, created_at";

#[async_trait]
impl InformationRepository for PgInformationRepository {
    async fn create_secret(&self, new: NewInformation<'_>) -> Result<Uuid, DomainError> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO influence_information \
             (guild_id, owner_id, target_user_id, target_username, content, visibility, veracity) \
             VALUES ($1, $2, $3, $4, $5, 'secret', 'vrai') RETURNING id",
        )
        .bind(new.guild_id)
        .bind(new.owner_id)
        .bind(new.target_user_id)
        .bind(new.target_username)
        .bind(new.content)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| pg_err_ctx("influence_information", e))?;
        Ok(id)
    }

    async fn get(&self, id: Uuid) -> Result<Option<Information>, DomainError> {
        let sql = format!("SELECT {INFO_COLS} FROM influence_information WHERE id = $1");
        let row: Option<InfoRow> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| pg_err_ctx("influence_information", e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list_secret_for_owner(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<Information>, DomainError> {
        let sql = format!(
            "SELECT {INFO_COLS} FROM influence_information \
             WHERE owner_id = $1 AND revealed = FALSE ORDER BY created_at DESC"
        );
        let rows: Vec<InfoRow> = sqlx::query_as(&sql)
            .bind(owner_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| pg_err_ctx("influence_information", e))?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn reveal(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE influence_information SET revealed = TRUE, visibility = 'public' WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| pg_err_ctx("influence_information", e))?;
        Ok(())
    }
}

// ── Archives ──

pub struct PgArchiveRepository {
    pool: PgPool,
}
impl PgArchiveRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ArchiveRow {
    event_type: String,
    payload: serde_json::Value,
    occurred_at: DateTime<Utc>,
}

#[async_trait]
impl ArchiveRepository for PgArchiveRepository {
    async fn append(
        &self,
        guild_id: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO influence_archives (guild_id, event_type, payload) VALUES ($1, $2, $3)",
        )
        .bind(guild_id)
        .bind(event_type)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| pg_err_ctx("influence_archives", e))?;
        Ok(())
    }

    async fn list_recent(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<sentinel_core::domain::entities::influence::archive::ArchiveEntry>, DomainError>
    {
        let rows: Vec<ArchiveRow> = sqlx::query_as(
            "SELECT event_type, payload, occurred_at FROM influence_archives \
             WHERE guild_id = $1 ORDER BY occurred_at DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| pg_err_ctx("influence_archives", e))?;
        Ok(rows
            .into_iter()
            .map(|r| sentinel_core::domain::entities::influence::archive::ArchiveEntry {
                event_type: r.event_type,
                payload: r.payload,
                occurred_at: r.occurred_at,
            })
            .collect())
    }
}
