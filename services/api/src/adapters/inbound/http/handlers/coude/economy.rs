//! Handlers économie : transferts inter-joueurs, vol, casino et compteurs
//! quotidiens. Délèguent à `state.coude_economy_uc`.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use super::dto::{GainDto, LostDto, StealDto, TransferCoinsDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

// ── Transferts ──

/// POST /api/coude/{guild_id}/transfer
pub async fn transfer_coins(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<TransferCoinsDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_economy_uc
        .transfer(&guild_id, &dto.from_id, &dto.to_id, dto.amount)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/steal
pub async fn record_steal(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<StealDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_economy_uc
        .steal(&guild_id, &dto.thief_id, &dto.victim_id, dto.amount)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Casino ──

/// POST /api/coude/{guild_id}/players/{user_id}/casino-win
pub async fn record_casino_win(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<GainDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_economy_uc
        .record_casino_win(&guild_id, &user_id, dto.gain)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/casino-loss
pub async fn record_casino_loss(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<LostDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_economy_uc
        .record_casino_loss(&guild_id, &user_id, dto.lost)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/casino-faillite
pub async fn record_casino_faillite(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let total_lost = state
        .coude_economy_uc
        .record_casino_faillite(&guild_id, &user_id)
        .await?;
    Ok(Json(serde_json::json!({ "total_lost": total_lost })))
}

/// GET /api/coude/{guild_id}/players/{user_id}/casino-today
pub async fn count_casino_today(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let count = state
        .coude_economy_uc
        .count_casino_today(&guild_id, &user_id)
        .await?;
    Ok(Json(serde_json::json!({ "count": count })))
}

/// GET /api/coude/{guild_id}/players/{user_id}/casino-gains-today
pub async fn sum_casino_gains_today(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let total = state
        .coude_economy_uc
        .sum_casino_gains_today(&guild_id, &user_id)
        .await?;
    Ok(Json(serde_json::json!({ "total": total })))
}

/// GET /api/coude/{guild_id}/players/{user_id}/steal-today
pub async fn count_steal_today(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let count = state
        .coude_economy_uc
        .count_steal_today(&guild_id, &user_id)
        .await?;
    Ok(Json(serde_json::json!({ "count": count })))
}
