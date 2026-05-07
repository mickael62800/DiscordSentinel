//! Job : suppression des anciennes donnees selon les retentions
//! configurees (voice_sessions, logs, audit_logs, ticket_messages, ...).
//!
//! Porte de cleanup-worker (Phase 1 fusion) — logique inchangee, seul
//! le chemin d'import a ete adapte pour pointer vers la config du
//! sentinel-worker.

use sqlx::PgPool;
use tracing::{info, warn};

use crate::config::CleanupConfig;

pub async fn run(pool: &PgPool, config: &CleanupConfig) -> Result<(), String> {
    let mut errors = Vec::new();

    // ── Voice sessions ──
    let voice_deleted = match sqlx::query(
        "DELETE FROM voice_sessions WHERE created_at < NOW() - make_interval(days => $1)",
    )
    .bind(config.voice_sessions_retention_days)
    .execute(pool)
    .await
    {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            warn!(error = %e, "Erreur suppression voice_sessions");
            errors.push(format!("voice_sessions: {e}"));
            0
        }
    };

    // ── Logs ──
    let logs_deleted = match sqlx::query(
        "DELETE FROM logs WHERE created_at < NOW() - make_interval(days => $1)",
    )
    .bind(config.logs_retention_days)
    .execute(pool)
    .await
    {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            warn!(error = %e, "Erreur suppression logs");
            errors.push(format!("logs: {e}"));
            0
        }
    };

    // ── Ticket messages from closed tickets ──
    let ticket_msgs_deleted = match sqlx::query(
        "DELETE FROM ticket_messages WHERE ticket_id IN (
            SELECT id FROM tickets WHERE status = 'closed'
            AND updated_at < NOW() - make_interval(days => $1)
        )",
    )
    .bind(config.closed_tickets_retention_days)
    .execute(pool)
    .await
    {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            warn!(error = %e, "Erreur suppression ticket_messages");
            errors.push(format!("ticket_messages: {e}"));
            0
        }
    };

    // ── Audit logs ──
    let audit_deleted = match sqlx::query(
        "DELETE FROM audit_logs WHERE created_at < NOW() - make_interval(days => $1)",
    )
    .bind(config.logs_retention_days)
    .execute(pool)
    .await
    {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            warn!(error = %e, "Erreur suppression audit_logs");
            errors.push(format!("audit_logs: {e}"));
            0
        }
    };

    // ── User activity log ──
    let activity_deleted = match sqlx::query(
        "DELETE FROM user_activity_log WHERE created_at < NOW() - make_interval(days => $1)",
    )
    .bind(config.logs_retention_days)
    .execute(pool)
    .await
    {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            warn!(error = %e, "Erreur suppression user_activity_log");
            errors.push(format!("user_activity_log: {e}"));
            0
        }
    };

    info!(
        voice_sessions = voice_deleted,
        logs = logs_deleted,
        ticket_messages = ticket_msgs_deleted,
        audit_logs = audit_deleted,
        user_activity_log = activity_deleted,
        "Cleaned {} voice_sessions, {} logs, {} ticket_messages, {} audit_logs, {} user_activity_log",
        voice_deleted,
        logs_deleted,
        ticket_msgs_deleted,
        audit_deleted,
        activity_deleted,
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Erreurs partielles: {}", errors.join("; ")))
    }
}
