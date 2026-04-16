//! Service d'export : queries + serialisation CSV/JSON.
//! Migre depuis export-worker pour centraliser la logique metier cote API.



use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::errors::DomainError;

/// Resultat d'un export : donnees serialisees + nombre de lignes.
pub struct ExportResult {
    pub data: String,
    pub row_count: usize,
}

#[async_trait]
pub trait ExecuteExportUseCase: Send + Sync {
    async fn execute(
        &self,
        guild_id: &str,
        job_type: &str,
        format: &str,
        max_rows: i64,
    ) -> Result<ExportResult, DomainError>;
}

pub struct ExportService {
    pool: PgPool,
}

impl ExportService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ExecuteExportUseCase for ExportService {
    async fn execute(
        &self,
        guild_id: &str,
        job_type: &str,
        format: &str,
        max_rows: i64,
    ) -> Result<ExportResult, DomainError> {
        let max_rows = max_rows.min(50_000).max(1);
        match job_type {
            "infractions" => export_infractions(&self.pool, guild_id, format, max_rows).await,
            "audit_logs" => export_audit_logs(&self.pool, guild_id, format, max_rows).await,
            "moderation_actions" => export_moderation_actions(&self.pool, guild_id, format, max_rows).await,
            other => Err(DomainError::ValidationError(format!("job_type inconnu: {other}"))),
        }
    }
}

// ═══════════════════════════════════════════════════
// Exporters par type
// ═══════════════════════════════════════════════════

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct InfractionRow {
    id: Uuid, guild_id: String, channel_id: String, user_id: String,
    username: String, message_id: String, content: String, score: f64,
    action: String, reason: String, duration: Option<i64>, created_at: DateTime<Utc>,
}

async fn export_infractions(pool: &PgPool, guild_id: &str, format: &str, max_rows: i64) -> Result<ExportResult, DomainError> {
    let rows: Vec<InfractionRow> = sqlx::query_as(
        "SELECT id, guild_id, channel_id, user_id, username, message_id, content, \
                score, action, reason, duration, created_at \
         FROM infractions WHERE guild_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(guild_id).bind(max_rows)
    .fetch_all(pool).await
    .map_err(|e| DomainError::Internal(format!("query infractions: {e}")))?;

    serialize_rows(&rows, format, |r| vec![
        r.id.to_string(), r.channel_id.clone(), r.user_id.clone(), r.username.clone(),
        r.message_id.clone(), r.content.clone(), format!("{:.3}", r.score), r.action.clone(),
        r.reason.clone(), r.duration.map(|d| d.to_string()).unwrap_or_default(), r.created_at.to_rfc3339(),
    ], &["id","channel_id","user_id","username","message_id","content","score","action","reason","duration_secs","created_at"])
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct AuditLogRow {
    id: Uuid, guild_id: String, event_type: String,
    actor_id: Option<String>, actor_name: Option<String>,
    target_id: Option<String>, target_name: Option<String>,
    channel_id: Option<String>, channel_name: Option<String>,
    created_at: DateTime<Utc>,
}

async fn export_audit_logs(pool: &PgPool, guild_id: &str, format: &str, max_rows: i64) -> Result<ExportResult, DomainError> {
    let rows: Vec<AuditLogRow> = sqlx::query_as(
        "SELECT id, guild_id, event_type, actor_id, actor_name, target_id, target_name, \
                channel_id, channel_name, created_at \
         FROM audit_logs WHERE guild_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(guild_id).bind(max_rows)
    .fetch_all(pool).await
    .map_err(|e| DomainError::Internal(format!("query audit_logs: {e}")))?;

    serialize_rows(&rows, format, |r| vec![
        r.id.to_string(), r.event_type.clone(),
        r.actor_id.clone().unwrap_or_default(), r.actor_name.clone().unwrap_or_default(),
        r.target_id.clone().unwrap_or_default(), r.target_name.clone().unwrap_or_default(),
        r.channel_id.clone().unwrap_or_default(), r.channel_name.clone().unwrap_or_default(),
        r.created_at.to_rfc3339(),
    ], &["id","event_type","actor_id","actor_name","target_id","target_name","channel_id","channel_name","created_at"])
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct ModerationActionRow {
    id: Uuid, guild_id: String, moderator_id: String, moderator_name: String,
    target_id: String, target_name: String, action_type: String,
    reason: String, duration: Option<i64>, created_at: DateTime<Utc>,
}

async fn export_moderation_actions(pool: &PgPool, guild_id: &str, format: &str, max_rows: i64) -> Result<ExportResult, DomainError> {
    let rows: Vec<ModerationActionRow> = sqlx::query_as(
        "SELECT id, guild_id, moderator_id, moderator_name, target_id, target_name, \
                action_type, reason, duration, created_at \
         FROM moderation_actions WHERE guild_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(guild_id).bind(max_rows)
    .fetch_all(pool).await
    .map_err(|e| DomainError::Internal(format!("query moderation_actions: {e}")))?;

    serialize_rows(&rows, format, |r| vec![
        r.id.to_string(), r.moderator_id.clone(), r.moderator_name.clone(),
        r.target_id.clone(), r.target_name.clone(), r.action_type.clone(),
        r.reason.clone(), r.duration.map(|d| d.to_string()).unwrap_or_default(), r.created_at.to_rfc3339(),
    ], &["id","moderator_id","moderator_name","target_id","target_name","action_type","reason","duration_secs","created_at"])
}

// ═══════════════════════════════════════════════════
// Serialisation helpers
// ═══════════════════════════════════════════════════

fn serialize_rows<T, F>(
    rows: &[T], format: &str, to_csv_row: F, headers: &[&str],
) -> Result<ExportResult, DomainError>
where
    T: serde::Serialize,
    F: Fn(&T) -> Vec<String>,
{
    let count = rows.len();
    let data = match format {
        "json" => serde_json::to_string(rows)
            .map_err(|e| DomainError::Internal(format!("json serialize: {e}")))?,
        "csv" => to_csv(rows, headers, to_csv_row),
        other => return Err(DomainError::ValidationError(format!("format inconnu: {other}"))),
    };
    Ok(ExportResult { data, row_count: count })
}

fn to_csv<T, F>(rows: &[T], headers: &[&str], to_row: F) -> String
where F: Fn(&T) -> Vec<String> {
    let mut out = String::new();
    out.push_str(&headers.join(","));
    out.push('\n');
    for row in rows {
        let line = to_row(row).iter().map(|c| csv_escape(c)).collect::<Vec<_>>().join(",");
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escape_simple() { assert_eq!(csv_escape("hello"), "hello"); }

    #[test]
    fn csv_escape_comma() { assert_eq!(csv_escape("a,b"), "\"a,b\""); }

    #[test]
    fn csv_escape_quote() { assert_eq!(csv_escape("a\"b"), "\"a\"\"b\""); }

    #[test]
    fn csv_escape_newline() { assert_eq!(csv_escape("a\nb"), "\"a\nb\""); }

    #[test]
    fn to_csv_basic() {
        let rows = vec![("1".to_string(), "hello".to_string()), ("2".to_string(), "a,b".to_string())];
        let csv = to_csv(&rows, &["id", "val"], |r| vec![r.0.clone(), r.1.clone()]);
        assert_eq!(csv, "id,val\n1,hello\n2,\"a,b\"\n");
    }

    #[test]
    fn to_csv_empty() {
        let rows: Vec<(String,)> = vec![];
        let csv = to_csv(&rows, &["id"], |r| vec![r.0.clone()]);
        assert_eq!(csv, "id\n");
    }

    #[test]
    fn serialize_rows_json() {
        let rows = vec![serde_json::json!({"a": 1}), serde_json::json!({"a": 2})];
        let result = serialize_rows(&rows, "json", |_| vec![], &[]).unwrap();
        assert_eq!(result.row_count, 2);
        assert!(result.data.contains("\"a\":1") || result.data.contains("\"a\": 1"));
    }

    #[test]
    fn serialize_rows_unknown_format() {
        let rows: Vec<String> = vec![];
        let result = serialize_rows(&rows, "xml", |_| vec![], &[]);
        assert!(result.is_err());
    }
}
