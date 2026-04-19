//! Handlers économie : transferts inter-joueurs, vol, casino et compteurs
//! quotidiens. Délèguent à `state.coude_economy_uc`.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::dto::{GainDto, LostDto, StealDto, TransferCoinsDto};
use super::taunts::TauntEventDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

// ── Transferts ──

/// Reponse du POST /api/coude/{guild_id}/transfer apres la migration
/// wallet unifie : expose les TauntEvents declenches (faillite, jackpot,
/// don genereux) pour que le bot les dispatche en un seul aller-retour.
#[derive(Debug, Serialize)]
pub struct TransferCoinsResponse {
    pub taunt_events: Vec<TauntEventDto>,
}

/// POST /api/coude/{guild_id}/transfer
pub async fn transfer_coins(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<TransferCoinsDto>,
) -> Result<Json<TransferCoinsResponse>, ApiError> {
    let taunts = state
        .coude_economy_uc
        .transfer(&guild_id, &dto.from_id, &dto.to_id, dto.amount)
        .await?;
    Ok(Json(TransferCoinsResponse {
        taunt_events: taunts.into_iter().map(Into::into).collect(),
    }))
}

/// Reponse du POST /api/coude/{guild_id}/steal apres la migration
/// wallet unifie : inclut le montant effectivement vole (clamp au solde
/// victime) + les TauntEvents declenches (faillite victime, jackpot
/// voleur).
#[derive(Debug, Serialize)]
pub struct StealResponse {
    pub stolen: i64,
    pub taunt_events: Vec<TauntEventDto>,
}

/// POST /api/coude/{guild_id}/steal
pub async fn record_steal(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<StealDto>,
) -> Result<Json<StealResponse>, ApiError> {
    let outcome = state
        .coude_economy_uc
        .steal(&guild_id, &dto.thief_id, &dto.victim_id, dto.amount)
        .await?;
    Ok(Json(StealResponse {
        stolen: outcome.stolen,
        taunt_events: outcome.taunt_events.into_iter().map(Into::into).collect(),
    }))
}

/// Payload POST /api/coude/{guild_id}/steal-fail-penalty.
#[derive(Debug, Deserialize)]
pub struct StealFailPenaltyDto {
    pub thief_id: String,
    pub amount: i64,
}

/// Reponse du POST /api/coude/{guild_id}/steal-fail-penalty.
#[derive(Debug, Serialize)]
pub struct StealFailPenaltyResponse {
    pub lost: i64,
    pub taunt_events: Vec<TauntEventDto>,
}

/// POST /api/coude/{guild_id}/steal-fail-penalty (migration wallet unifie).
pub async fn steal_fail_penalty(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<StealFailPenaltyDto>,
) -> Result<Json<StealFailPenaltyResponse>, ApiError> {
    let (lost, taunts) = state
        .coude_economy_uc
        .steal_fail_penalty(&guild_id, &dto.thief_id, dto.amount)
        .await?;
    Ok(Json(StealFailPenaltyResponse {
        lost,
        taunt_events: taunts.into_iter().map(Into::into).collect(),
    }))
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
