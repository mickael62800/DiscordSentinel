use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::audit::audit_log::AuditLog;
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::audit::manage_audit_logs::AuditLogFilters;
use crate::ports::outbound::audit::audit_log_repository::AuditLogRepository;

pub struct PgAuditLogRepository {
    pool: PgPool,
}

impl PgAuditLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AuditLogRow {
    id: Uuid,
    guild_id: String,
    event_type: String,
    actor_id: Option<String>,
    actor_name: Option<String>,
    target_id: Option<String>,
    target_name: Option<String>,
    channel_id: Option<String>,
    channel_name: Option<String>,
    details: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<AuditLogRow> for AuditLog {
    fn from(row: AuditLogRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id.into(),
            event_type: row.event_type,
            actor_id: row.actor_id,
            actor_name: row.actor_name,
            target_id: row.target_id.into(),
            target_name: row.target_name,
            channel_id: row.channel_id.into(),
            channel_name: row.channel_name,
            details: row.details,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl AuditLogRepository for PgAuditLogRepository {
    async fn save(&self, log: &AuditLog) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO audit_logs (id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, channel_name, details, created_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
        )
        .bind(log.id)
        .bind(log.guild_id.as_str())
        .bind(&log.event_type)
        .bind(&log.actor_id)
        .bind(&log.actor_name)
        .bind(log.target_id.as_deref())
        .bind(&log.target_name)
        .bind(log.channel_id.as_deref())
        .bind(&log.channel_name)
        .bind(&log.details)
        .bind(log.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn find_all(
        &self,
        guild_id: Option<&str>,
        filters: &AuditLogFilters,
    ) -> Result<Vec<AuditLog>, DomainError> {
        let mut query = String::from("SELECT * FROM audit_logs WHERE 1=1");
        let mut param_idx = 1u32;

        if guild_id.is_some() {
            query.push_str(&format!(" AND guild_id = ${param_idx}"));
            param_idx += 1;
        }
        if filters.event_type.is_some() {
            query.push_str(&format!(" AND event_type = ${param_idx}"));
            param_idx += 1;
        }
        if filters.actor_id.is_some() {
            query.push_str(&format!(" AND actor_id = ${param_idx}"));
            param_idx += 1;
        }
        if filters.target_id.is_some() {
            query.push_str(&format!(" AND target_id = ${param_idx}"));
            param_idx += 1;
        }

        query.push_str(&format!(" ORDER BY created_at DESC LIMIT ${param_idx}"));
        param_idx += 1;
        query.push_str(&format!(" OFFSET ${param_idx}"));

        let mut q = sqlx::query_as::<_, AuditLogRow>(&query);

        if let Some(gid) = guild_id {
            q = q.bind(gid);
        }
        if let Some(ref et) = filters.event_type {
            q = q.bind(et);
        }
        if let Some(ref aid) = filters.actor_id {
            q = q.bind(aid);
        }
        if let Some(ref tid) = filters.target_id {
            q = q.bind(tid);
        }

        q = q.bind(filters.limit).bind(filters.offset);

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(AuditLog::from).collect())
    }

    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM audit_logs WHERE guild_id = $1 AND created_at < NOW() - make_interval(days => $2)")
            .bind(guild_id)
            .bind(days)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(format!("delete_audit_logs_older: {e}")))?;
        Ok(result.rows_affected())
    }
}
