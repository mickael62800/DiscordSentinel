//! Phase 6A — Drain de la file d'attente export_jobs.
//!
//! Flow :
//!   1. Reset les jobs `processing` zombies (> PROCESSING_TIMEOUT_SECS) -> `pending`
//!   2. Claim 1 job via `UPDATE ... FOR UPDATE SKIP LOCKED RETURNING` (atomic)
//!   3. Execute la query selon job_type + serialize selon format
//!   4. UPDATE status = 'done' + result + result_rows (ou 'failed'/'dead' si retry max)
//!
//! Le claim atomique permet de scaler horizontalement l'export-worker sans
//! collision. On traite 1 job par tick pour ne pas bloquer les autres jobs
//! en cas de gros export (next tick dans scan_interval_secs).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::{MAX_ROWS_PER_EXPORT, PROCESSING_TIMEOUT_SECS};

#[derive(Debug, sqlx::FromRow)]
struct ClaimedJob {
    id: Uuid,
    guild_id: String,
    job_type: String,
    format: String,
    filters: serde_json::Value,
    retries: i32,
    max_retries: i32,
}

pub async fn run(pool: &PgPool) -> Result<(), String> {
    // 1. Reset les jobs zombies
    let reset = sqlx::query(
        "UPDATE export_jobs SET status = 'pending', started_at = NULL \
         WHERE status = 'processing' \
           AND started_at < NOW() - make_interval(secs => $1)",
    )
    .bind(PROCESSING_TIMEOUT_SECS)
    .execute(pool)
    .await
    .map_err(|e| format!("reset zombies: {e}"))?;
    if reset.rows_affected() > 0 {
        warn!(count = reset.rows_affected(), "Export jobs zombies reset");
    }

    // 2. Claim 1 job pending atomiquement
    let claimed: Option<ClaimedJob> = sqlx::query_as::<_, ClaimedJob>(
        "UPDATE export_jobs SET status = 'processing', started_at = NOW() \
         WHERE id = ( \
             SELECT id FROM export_jobs \
             WHERE status = 'pending' \
             ORDER BY created_at ASC \
             FOR UPDATE SKIP LOCKED \
             LIMIT 1 \
         ) \
         RETURNING id, guild_id, job_type, format, filters, retries, max_retries",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("claim job: {e}"))?;

    let Some(job) = claimed else {
        debug!("Aucun export job pending");
        return Ok(());
    };

    info!(
        job_id = %job.id,
        guild_id = %job.guild_id,
        job_type = %job.job_type,
        format = %job.format,
        "Export job claim"
    );

    // 3. Executer l'export
    let result = match job.job_type.as_str() {
        "infractions" => export_infractions(pool, &job.guild_id, &job.format, &job.filters).await,
        "audit_logs" => export_audit_logs(pool, &job.guild_id, &job.format, &job.filters).await,
        "moderation_actions" => {
            export_moderation_actions(pool, &job.guild_id, &job.format, &job.filters).await
        }
        other => Err(format!("job_type inconnu: {other}")),
    };

    // 4. Persister le resultat
    match result {
        Ok((serialized, rows)) => {
            sqlx::query(
                "UPDATE export_jobs SET status = 'done', result = $1, result_rows = $2, completed_at = NOW() \
                 WHERE id = $3",
            )
            .bind(&serialized)
            .bind(rows as i32)
            .bind(job.id)
            .execute(pool)
            .await
            .map_err(|e| format!("mark done: {e}"))?;

            info!(
                job_id = %job.id,
                rows,
                bytes = serialized.len(),
                "Export job done"
            );
        }
        Err(err) => {
            let new_retries = job.retries + 1;
            let dead = new_retries >= job.max_retries;
            let new_status = if dead { "dead" } else { "failed" };

            sqlx::query(
                "UPDATE export_jobs SET status = $1, retries = $2, error_message = $3, completed_at = NOW() \
                 WHERE id = $4",
            )
            .bind(new_status)
            .bind(new_retries)
            .bind(&err)
            .bind(job.id)
            .execute(pool)
            .await
            .map_err(|e| format!("mark failed: {e}"))?;

            warn!(job_id = %job.id, error = %err, retries = new_retries, dead, "Export job failed");
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════
// Exporters par type de donnees
// ═══════════════════════════════════════════════════

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct InfractionRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    user_name: Option<String>,
    action: String,
    reason: Option<String>,
    moderator_id: Option<String>,
    moderator_name: Option<String>,
    created_at: DateTime<Utc>,
}

async fn export_infractions(
    pool: &PgPool,
    guild_id: &str,
    format: &str,
    _filters: &serde_json::Value,
) -> Result<(String, usize), String> {
    let rows: Vec<InfractionRow> = sqlx::query_as::<_, InfractionRow>(
        "SELECT id, guild_id, user_id, user_name, action, reason, moderator_id, moderator_name, created_at \
         FROM infractions \
         WHERE guild_id = $1 \
         ORDER BY created_at DESC \
         LIMIT $2",
    )
    .bind(guild_id)
    .bind(MAX_ROWS_PER_EXPORT)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query infractions: {e}"))?;

    serialize_rows(&rows, format, |r| {
        vec![
            r.id.to_string(),
            r.user_id.clone(),
            r.user_name.clone().unwrap_or_default(),
            r.action.clone(),
            r.reason.clone().unwrap_or_default(),
            r.moderator_id.clone().unwrap_or_default(),
            r.moderator_name.clone().unwrap_or_default(),
            r.created_at.to_rfc3339(),
        ]
    }, &[
        "id", "user_id", "user_name", "action", "reason",
        "moderator_id", "moderator_name", "created_at",
    ])
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
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
    created_at: DateTime<Utc>,
}

async fn export_audit_logs(
    pool: &PgPool,
    guild_id: &str,
    format: &str,
    _filters: &serde_json::Value,
) -> Result<(String, usize), String> {
    let rows: Vec<AuditLogRow> = sqlx::query_as::<_, AuditLogRow>(
        "SELECT id, guild_id, event_type, actor_id, actor_name, target_id, target_name, \
                channel_id, channel_name, created_at \
         FROM audit_logs \
         WHERE guild_id = $1 \
         ORDER BY created_at DESC \
         LIMIT $2",
    )
    .bind(guild_id)
    .bind(MAX_ROWS_PER_EXPORT)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query audit_logs: {e}"))?;

    serialize_rows(&rows, format, |r| {
        vec![
            r.id.to_string(),
            r.event_type.clone(),
            r.actor_id.clone().unwrap_or_default(),
            r.actor_name.clone().unwrap_or_default(),
            r.target_id.clone().unwrap_or_default(),
            r.target_name.clone().unwrap_or_default(),
            r.channel_id.clone().unwrap_or_default(),
            r.channel_name.clone().unwrap_or_default(),
            r.created_at.to_rfc3339(),
        ]
    }, &[
        "id", "event_type", "actor_id", "actor_name", "target_id",
        "target_name", "channel_id", "channel_name", "created_at",
    ])
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct ModerationActionRow {
    id: Uuid,
    guild_id: String,
    moderator_id: String,
    moderator_name: String,
    target_id: String,
    target_name: String,
    action_type: String,
    reason: String,
    duration: Option<i64>,
    created_at: DateTime<Utc>,
}

async fn export_moderation_actions(
    pool: &PgPool,
    guild_id: &str,
    format: &str,
    _filters: &serde_json::Value,
) -> Result<(String, usize), String> {
    let rows: Vec<ModerationActionRow> = sqlx::query_as::<_, ModerationActionRow>(
        "SELECT id, guild_id, moderator_id, moderator_name, target_id, target_name, \
                action_type, reason, duration, created_at \
         FROM moderation_actions \
         WHERE guild_id = $1 \
         ORDER BY created_at DESC \
         LIMIT $2",
    )
    .bind(guild_id)
    .bind(MAX_ROWS_PER_EXPORT)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query moderation_actions: {e}"))?;

    serialize_rows(&rows, format, |r| {
        vec![
            r.id.to_string(),
            r.moderator_id.clone(),
            r.moderator_name.clone(),
            r.target_id.clone(),
            r.target_name.clone(),
            r.action_type.clone(),
            r.reason.clone(),
            r.duration.map(|d| d.to_string()).unwrap_or_default(),
            r.created_at.to_rfc3339(),
        ]
    }, &[
        "id", "moderator_id", "moderator_name", "target_id", "target_name",
        "action_type", "reason", "duration_secs", "created_at",
    ])
}

// ═══════════════════════════════════════════════════
// Serialization helpers
// ═══════════════════════════════════════════════════

fn serialize_rows<T, F>(
    rows: &[T],
    format: &str,
    to_csv_row: F,
    headers: &[&str],
) -> Result<(String, usize), String>
where
    T: serde::Serialize,
    F: Fn(&T) -> Vec<String>,
{
    let count = rows.len();
    let serialized = match format {
        "json" => serde_json::to_string(rows).map_err(|e| format!("json serialize: {e}"))?,
        "csv" => to_csv(rows, headers, to_csv_row),
        other => return Err(format!("format inconnu: {other}")),
    };
    Ok((serialized, count))
}

/// Genere un CSV en escapant les champs contenant `,`, `"` ou `\n`.
fn to_csv<T, F>(rows: &[T], headers: &[&str], to_row: F) -> String
where
    F: Fn(&T) -> Vec<String>,
{
    let mut out = String::new();
    out.push_str(&headers.join(","));
    out.push('\n');
    for row in rows {
        let cols = to_row(row);
        let line = cols
            .iter()
            .map(|c| csv_escape(c))
            .collect::<Vec<_>>()
            .join(",");
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escape_simple() {
        assert_eq!(csv_escape("hello"), "hello");
    }

    #[test]
    fn csv_escape_comma() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
    }

    #[test]
    fn csv_escape_quote() {
        assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn csv_escape_newline() {
        assert_eq!(csv_escape("a\nb"), "\"a\nb\"");
    }

    #[test]
    fn to_csv_basic() {
        struct R {
            a: String,
            b: String,
        }
        let rows = vec![
            R { a: "1".into(), b: "hello".into() },
            R { a: "2".into(), b: "a,b".into() },
        ];
        let out = to_csv(&rows, &["a", "b"], |r| vec![r.a.clone(), r.b.clone()]);
        assert_eq!(out, "a,b\n1,hello\n2,\"a,b\"\n");
    }
}
