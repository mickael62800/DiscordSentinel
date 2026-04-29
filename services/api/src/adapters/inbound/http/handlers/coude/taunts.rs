//! Handlers HTTP pour la config des railleries (Phase 9 Part E).
//!
//! 3 endpoints :
//!   - GET    /api/coude/{guild_id}/config/taunts        (tous les users)
//!   - PUT    /api/coude/{guild_id}/config/taunts        (Admin+)
//!   - DELETE /api/coude/{guild_id}/config/taunts/opt-outs/{user_id}
//!                                                       (Admin+ : retrait force)
//!
//! Logique metier zero : on ne fait que deleguer au use case.

use axum::extract::Extension;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::require_role;
use crate::domain::enums::system::role::Role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::ChannelId;
use crate::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Serialize)]
pub struct TauntEventDto {
    pub channel_id: ChannelId,
    pub target_user_id: String,
    pub message: String,
    pub nickname_suffix: String,
    pub streak_kind: String,
    pub streak_value: i32,
}

impl From<TauntEvent> for TauntEventDto {
    fn from(e: TauntEvent) -> Self {
        Self {
            channel_id: e.channel_id,
            target_user_id: e.target_user_id,
            message: e.message,
            nickname_suffix: e.nickname_suffix,
            streak_kind: e.streak_kind.to_string(),
            streak_value: e.streak_value,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MaybeTauntEventDto {
    pub event: Option<TauntEventDto>,
}

#[derive(Debug, Deserialize)]
pub struct EcoAmountDto {
    pub amount: i64,
}

#[derive(Debug, Serialize)]
pub struct TauntsConfigDto {
    pub guild_id: GuildId,
    pub channel_id: Option<String>,
    pub enabled: bool,
    pub rename_enabled: bool,
    pub messages_enabled: bool,
    pub opt_outs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTauntsConfigDto {
    pub channel_id: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub rename_enabled: Option<bool>,
    #[serde(default)]
    pub messages_enabled: Option<bool>,
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
        rename_enabled: config.rename_enabled,
        messages_enabled: config.messages_enabled,
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
    if let Some(rename_enabled) = dto.rename_enabled {
        state
            .coude_taunts_uc
            .set_rename_enabled(&guild_id, rename_enabled)
            .await?;
    }
    if let Some(messages_enabled) = dto.messages_enabled {
        state
            .coude_taunts_uc
            .set_messages_enabled(&guild_id, messages_enabled)
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

// ── Tracking endpoints (appeles par le bot apres une resolution de main) ──

/// POST /api/coude/{guild_id}/taunts/bj/natural/{user_id}
pub async fn track_bj_natural(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<MaybeTauntEventDto>, ApiError> {
    let ev = state
        .coude_taunts_uc
        .on_bj_natural(&guild_id, &user_id)
        .await?;
    Ok(Json(MaybeTauntEventDto {
        event: ev.map(Into::into),
    }))
}

/// POST /api/coude/{guild_id}/taunts/bj/won/{user_id}
pub async fn track_bj_won(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<MaybeTauntEventDto>, ApiError> {
    let ev = state
        .coude_taunts_uc
        .on_bj_hand_won(&guild_id, &user_id)
        .await?;
    Ok(Json(MaybeTauntEventDto {
        event: ev.map(Into::into),
    }))
}

/// POST /api/coude/{guild_id}/taunts/bj/bust/{user_id}
pub async fn track_bj_bust(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<MaybeTauntEventDto>, ApiError> {
    let ev = state
        .coude_taunts_uc
        .on_bj_hand_bust(&guild_id, &user_id)
        .await?;
    Ok(Json(MaybeTauntEventDto {
        event: ev.map(Into::into),
    }))
}

/// POST /api/coude/{guild_id}/taunts/eco/bankruptcy/{user_id}
pub async fn track_bankruptcy(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<MaybeTauntEventDto>, ApiError> {
    let ev = state
        .coude_taunts_uc
        .on_bankruptcy(&guild_id, &user_id)
        .await?;
    Ok(Json(MaybeTauntEventDto {
        event: ev.map(Into::into),
    }))
}

/// POST /api/coude/{guild_id}/taunts/eco/jackpot/{user_id}
pub async fn track_jackpot(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<EcoAmountDto>,
) -> Result<Json<MaybeTauntEventDto>, ApiError> {
    let ev = state
        .coude_taunts_uc
        .on_jackpot(&guild_id, &user_id, dto.amount)
        .await?;
    Ok(Json(MaybeTauntEventDto {
        event: ev.map(Into::into),
    }))
}

/// POST /api/coude/{guild_id}/taunts/eco/donor/{user_id}
pub async fn track_generous_donor(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<EcoAmountDto>,
) -> Result<Json<MaybeTauntEventDto>, ApiError> {
    let ev = state
        .coude_taunts_uc
        .on_generous_donor(&guild_id, &user_id, dto.amount)
        .await?;
    Ok(Json(MaybeTauntEventDto {
        event: ev.map(Into::into),
    }))
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

#[cfg(test)]
#[path = "tests/taunts.rs"]
mod tests;
