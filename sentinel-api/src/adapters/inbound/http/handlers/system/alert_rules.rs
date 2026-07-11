//! CRUD des regles d'alerte de supervision (table `alert_rules`).
//!
//! Endpoint host-level : reserve aux superadmins (comme docker/security).
//! Les regles pilotent `outbound/system/alerts_dispatcher.rs`.

use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::{require_superadmin, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::errors::DomainError;

fn forbid(msg: &str) -> ApiError {
    ApiError(DomainError::Forbidden(msg.into()))
}

fn gate_super(state: &AppState, rbac: &Option<Extension<RoleContext>>) -> Result<(), ApiError> {
    let Some(Extension(ctx)) = rbac else {
        return Err(forbid("auth requise"));
    };
    require_superadmin(state, ctx).map_err(|_| forbid("superadmin requis"))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct AlertRuleDto {
    pub id: String,
    pub label: String,
    pub metric: String,
    pub comparator: String,
    pub threshold: Option<f64>,
    pub enabled: bool,
    pub severity: String,
    pub cooldown_secs: i32,
}

const SELECT_COLS: &str =
    "id, label, metric, comparator, threshold, enabled, severity, cooldown_secs";

/// GET /api/alert-rules — liste toutes les regles (actives ou non).
pub async fn list_alert_rules(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<Vec<AlertRuleDto>>, ApiError> {
    gate_super(&state, &rbac)?;
    let rows = sqlx::query_as::<_, AlertRuleDto>(&format!(
        "SELECT {SELECT_COLS} FROM alert_rules ORDER BY id"
    ))
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct UpdateAlertRuleDto {
    pub enabled: Option<bool>,
    pub threshold: Option<f64>,
    pub severity: Option<String>,
    pub cooldown_secs: Option<i32>,
}

/// PATCH /api/alert-rules/{id} — met a jour les champs editables d'une regle.
/// `metric`/`comparator`/`label` sont fixes (ils definissent la semantique).
pub async fn update_alert_rule(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateAlertRuleDto>,
) -> Result<Json<AlertRuleDto>, ApiError> {
    gate_super(&state, &rbac)?;

    if let Some(ref s) = dto.severity {
        if !matches!(s.as_str(), "info" | "warning" | "critical") {
            return Err(ApiError(DomainError::ValidationError(
                "severite invalide (info|warning|critical)".into(),
            )));
        }
    }
    if let Some(c) = dto.cooldown_secs {
        if c < 60 {
            return Err(ApiError(DomainError::ValidationError(
                "cooldown_secs minimum 60".into(),
            )));
        }
    }

    // COALESCE : seuls les champs fournis sont modifies. threshold n'est pas
    // remis a NULL par cet endpoint (les metriques booleennes le gardent NULL).
    let row = sqlx::query_as::<_, AlertRuleDto>(&format!(
        "UPDATE alert_rules SET \
         enabled = COALESCE($2, enabled), \
         threshold = COALESCE($3, threshold), \
         severity = COALESCE($4, severity), \
         cooldown_secs = COALESCE($5, cooldown_secs), \
         updated_at = NOW() \
         WHERE id = $1 RETURNING {SELECT_COLS}"
    ))
    .bind(&id)
    .bind(dto.enabled)
    .bind(dto.threshold)
    .bind(&dto.severity)
    .bind(dto.cooldown_secs)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?
    .ok_or_else(|| ApiError(DomainError::NotFound("regle d'alerte inconnue".into())))?;

    Ok(Json(row))
}
