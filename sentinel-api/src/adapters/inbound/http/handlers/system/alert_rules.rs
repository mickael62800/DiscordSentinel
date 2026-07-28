//! CRUD des regles d'alerte de supervision (table `alert_rules`).
//!
//! Endpoint host-level : reserve aux superadmins (comme docker/security).
//! Les regles pilotent `outbound/system/alerts_dispatcher.rs`. Le handler ne
//! fait que RBAC + mapping DTO ; invariants et SQL vivent derrière le use case
//! (`manage_alert_rules` + `PgAlertRuleRepository`).

use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::{require_superadmin, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::entities::system::alert_rule::{AlertRule, AlertRuleUpdate};
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

#[derive(Serialize)]
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

impl From<AlertRule> for AlertRuleDto {
    fn from(r: AlertRule) -> Self {
        Self {
            id: r.id,
            label: r.label,
            metric: r.metric,
            comparator: r.comparator,
            threshold: r.threshold,
            enabled: r.enabled,
            severity: r.severity,
            cooldown_secs: r.cooldown_secs,
        }
    }
}

/// GET /api/alert-rules — liste toutes les regles (actives ou non).
pub async fn list_alert_rules(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<Vec<AlertRuleDto>>, ApiError> {
    gate_super(&state, &rbac)?;
    let rules = state.alert_rules_uc.list().await?;
    Ok(Json(rules.into_iter().map(Into::into).collect()))
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
    let update = AlertRuleUpdate {
        enabled: dto.enabled,
        threshold: dto.threshold,
        severity: dto.severity,
        cooldown_secs: dto.cooldown_secs,
    };
    let rule = state.alert_rules_uc.update(&id, update).await?;
    Ok(Json(rule.into()))
}
