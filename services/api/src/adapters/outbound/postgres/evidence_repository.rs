use async_trait::async_trait;
use sqlx::PgPool;

use crate::ports::outbound::moderation::evidence_repository::EvidenceEntry;
use crate::ports::outbound::moderation::evidence_repository::EvidenceRepository;
use super::pg_err;

pub struct PgEvidenceRepository { pool: PgPool }

impl PgEvidenceRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: uuid::Uuid,
    url: String,
    description: Option<String>,
    uploaded_by: String,
    uploaded_by_name: String,
    uploaded_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
impl EvidenceRepository for PgEvidenceRepository {
    async fn add(
        &self, action_id: uuid::Uuid, url: &str, description: Option<&str>,
        uploaded_by: &str, uploaded_by_name: &str,
    ) -> Result<EvidenceEntry, crate::domain::errors::DomainError> {
        let row: Row = sqlx::query_as(
            "INSERT INTO moderation_evidence (action_id, url, description, uploaded_by, uploaded_by_name) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id, url, description, uploaded_by, uploaded_by_name, uploaded_at",
        )
        .bind(action_id).bind(url).bind(description).bind(uploaded_by).bind(uploaded_by_name)
        .fetch_one(&self.pool).await.map_err(pg_err)?;

        Ok(EvidenceEntry {
            id: row.id, action_id, url: row.url, description: row.description,
            uploaded_by: row.uploaded_by, uploaded_by_name: row.uploaded_by_name,
            uploaded_at: row.uploaded_at,
        })
    }

    async fn list(&self, action_id: uuid::Uuid) -> Result<Vec<EvidenceEntry>, crate::domain::errors::DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, url, description, uploaded_by, uploaded_by_name, uploaded_at \
             FROM moderation_evidence WHERE action_id = $1 ORDER BY uploaded_at ASC",
        )
        .bind(action_id).fetch_all(&self.pool).await.map_err(pg_err)?;

        Ok(rows.into_iter().map(|r| EvidenceEntry {
            id: r.id, action_id, url: r.url, description: r.description,
            uploaded_by: r.uploaded_by, uploaded_by_name: r.uploaded_by_name,
            uploaded_at: r.uploaded_at,
        }).collect())
    }
}
