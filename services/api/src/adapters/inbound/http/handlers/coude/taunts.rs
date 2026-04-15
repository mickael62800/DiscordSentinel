//! Handlers HTTP pour la config des railleries (Phase 9 Part E).
//!
//! 3 endpoints :
//!   - GET    /api/coude/{guild_id}/config/taunts        (tous les users)
//!   - PUT    /api/coude/{guild_id}/config/taunts        (Admin+)
//!   - DELETE /api/coude/{guild_id}/config/taunts/opt-outs/{user_id}
//!                                                       (Admin+ : retrait force)
//!
//! Logique metier zero : on ne fait que deleguer au use case.

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::{require_role, Role, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

#[derive(Debug, Serialize)]
pub struct TauntsConfigDto {
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub enabled: bool,
    pub opt_outs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTauntsConfigDto {
    pub channel_id: Option<String>,
    pub enabled: bool,
}

/// GET /api/coude/{guild_id}/config/taunts
pub async fn get_taunts_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<TauntsConfigDto>, ApiError> {
    let config = state.coude_taunts_uc.get_config(&guild_id).await?;
    let opt_outs = state.coude_taunts_uc.list_opt_outs(&guild_id).await?;
    Ok(Json(TauntsConfigDto {
        guild_id: config.guild_id,
        channel_id: config.channel_id,
        enabled: config.enabled,
        opt_outs,
    }))
}

/// PUT /api/coude/{guild_id}/config/taunts
pub async fn update_taunts_config(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(guild_id): Path<String>,
    Json(dto): Json<UpdateTauntsConfigDto>,
) -> Result<StatusCode, ApiError> {
    if let Some(Extension(ctx)) = rbac {
        require_role(&ctx, Role::Admin).map_err(|_| {
            ApiError::from(DomainError::Forbidden(
                "admin+ requis pour configurer les railleries".into(),
            ))
        })?;
    }
    state
        .coude_taunts_uc
        .set_channel(&guild_id, dto.channel_id.as_deref())
        .await?;
    state
        .coude_taunts_uc
        .set_enabled(&guild_id, dto.enabled)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/coude/{guild_id}/config/taunts/opt-outs/{user_id}
/// Retire un opt-out manuellement (action admin).
pub async fn remove_taunts_opt_out(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if let Some(Extension(ctx)) = rbac {
        require_role(&ctx, Role::Admin).map_err(|_| {
            ApiError::from(DomainError::Forbidden(
                "admin+ requis pour retirer un opt-out".into(),
            ))
        })?;
    }
    state
        .coude_taunts_uc
        .set_opt_out(&guild_id, &user_id, false)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
