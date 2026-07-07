//! Audit serveur : actions admin sur l'infra (vs audit_logs = events Discord).
//!
//! Adaptateur ENTRANT mince : le SQL vit dans `ServerEventRepository`, le bornage
//! des filtres dans `ManageServerEventsUseCase`. Ici : parse -> RBAC -> use case.
//! Helper `record_server_event` : ecriture best-effort (log l'erreur sans bloquer
//! l'action principale de l'appelant).

use std::sync::Arc;

use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::require_role;
use crate::adapters::inbound::http::middleware::rbac::require_superadmin;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::system::manage_server_events::ManageServerEventsUseCase;
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

/// Insere un event serveur via le use case. Best-effort : si echec, on log
/// l'erreur mais on ne bloque pas l'action principale qui appelle ce helper.
///
/// Severities :
/// - "info"     : action normale d'admin (start container, cleanup logs)
/// - "warn"     : action a surveiller (force prune, role grant a un nouveau)
/// - "critical" : action destructive importante (delete volume, prune system)
pub async fn record_server_event(
    uc: &Arc<dyn ManageServerEventsUseCase>,
    actor: &str,
    actor_name: Option<&str>,
    action: &str,
    target: Option<&str>,
    severity: &str,
    details: serde_json::Value,
) {
    if let Err(e) = uc
        .record(actor, actor_name, action, target, severity, details)
        .await
    {
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

    let events = state
        .server_events_uc
        .list(q.action_prefix, q.severity, q.limit)
        .await?;

    let out = events
        .into_iter()
        .map(|e| ServerEventDto {
            id: e.id,
            timestamp: e.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            actor: e.actor,
            actor_name: e.actor_name,
            action: e.action,
            target: e.target,
            severity: e.severity,
            details: e.details,
        })
        .collect();
    Ok(Json(out))
}
