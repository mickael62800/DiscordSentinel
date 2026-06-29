//! Audit serveur : actions admin sur l'infra (vs audit_logs = events Discord).
//!
//! Helper `record_server_event` insere une row dans la table `server_events`.
//! Endpoint `GET /api/security/server-events` lit la table avec filtres.

use crate::adapters::inbound::http::errors_helpers::sqlx_internal;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::require_role;
use crate::adapters::inbound::http::middleware::rbac::require_superadmin;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::errors::DomainError;

fn forbid(s: StatusCode, msg: &str) -> ApiError {
    ApiError(if s == StatusCode::FORBIDDEN {
        DomainError::Forbidden(msg.into())
    } else {
        DomainError::Internal(msg.into())
    })
}

fn gate_admin(state: &AppState, rbac: &Option<Extension<RoleContext>>) -> Result<(), ApiError> {
    let Some(Extension(ctx)) = rbac else {
        return Err(forbid(StatusCode::FORBIDDEN, "auth requise"));
    };
    if require_superadmin(state, ctx).is_ok() {
        return Ok(());
    }
    require_role(ctx, Role::Admin).map_err(|s| forbid(s, "admin+ requis"))
}

/// Insere un event serveur en BDD. Best-effort : si echec DB, on log l'erreur
/// mais on ne bloque pas l'action principale qui appelle ce helper.
///
/// Severities :
/// - "info"     : action normale d'admin (start container, cleanup logs)
/// - "warn"     : action a surveiller (force prune, role grant a un nouveau)
/// - "critical" : action destructive importante (delete volume, prune system)
pub async fn record_server_event(
    pool: &PgPool,
    actor: &str,
    actor_name: Option<&str>,
    action: &str,
    target: Option<&str>,
    severity: &str,
    details: serde_json::Value,
) {
    let res = sqlx::query(
        "INSERT INTO server_events (actor, actor_name, action, target, severity, details) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(actor)
    .bind(actor_name)
    .bind(action)
    .bind(target)
    .bind(severity)
    .bind(&details)
    .execute(pool)
    .await;
    if let Err(e) = res {
        tracing::warn!(error = %e, action = action, "Echec insert server_events");
    }
}

// ── Endpoint : lire les events ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ServerEventsQuery {
    pub action_prefix: Option<String>,
    pub severity: Option<String>, // "info" | "warn" | "critical"
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ServerEventDto {
    pub id: String,
    pub timestamp: String,
    pub actor: Option<String>,
    pub actor_name: Option<String>,
    pub action: String,
    pub target: Option<String>,
    pub severity: String,
    pub details: serde_json::Value,
}

pub async fn list_server_events(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(q): Query<ServerEventsQuery>,
) -> Result<Json<Vec<ServerEventDto>>, ApiError> {
    gate_admin(&state, &rbac)?;
    let limit = q.limit.unwrap_or(100).clamp(1, 500);

    let mut sql = String::from(
        "SELECT id::text, \
                to_char(timestamp, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'), \
                actor, actor_name, action, target, severity, details \
         FROM server_events WHERE 1=1",
    );
    let mut idx = 1;
    if q.action_prefix.is_some() {
        sql.push_str(&format!(" AND action LIKE ${idx} || '%'"));
        idx += 1;
    }
    if q.severity.is_some() {
        sql.push_str(&format!(" AND severity = ${idx}"));
        idx += 1;
    }
    sql.push_str(&format!(" ORDER BY timestamp DESC LIMIT ${idx}"));

    let mut q_builder = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
            String,
            serde_json::Value,
        ),
    >(&sql);
    if let Some(p) = &q.action_prefix {
        q_builder = q_builder.bind(p);
    }
    if let Some(s) = &q.severity {
        q_builder = q_builder.bind(s);
    }
    q_builder = q_builder.bind(limit);

    let rows = q_builder
        .fetch_all(&state.pg_pool)
        .await
        .map_err(sqlx_internal("query"))?;

    let out = rows
        .into_iter()
        .map(
            |(id, ts, actor, actor_name, action, target, severity, details)| ServerEventDto {
                id,
                timestamp: ts,
                actor,
                actor_name,
                action,
                target,
                severity,
                details,
            },
        )
        .collect();
    Ok(Json(out))
}
