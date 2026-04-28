use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::community::conduct::AddPointsDto;
use crate::adapters::inbound::http::dto::community::conduct::ConductConfigDto;
use crate::adapters::inbound::http::dto::community::conduct::ConductPointsLogDto;
use crate::adapters::inbound::http::dto::community::conduct::SaveConductConfigDto;
use crate::adapters::inbound::http::dto::community::conduct::UserConductPointsDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::normalize_limit;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::community::manage_conduct::AddPointsCommand;

#[derive(Deserialize)]
pub struct LeaderboardQuery {
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct LogQuery {
    pub limit: Option<i64>,
}

pub async fn get_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<ConductConfigDto>, ApiError> {
    let config = state.conduct_uc.get_config(&guild_id).await?;
    Ok(single_dto(config))
}

pub async fn save_config(
    State(state): State<AppState>,
    Json(dto): Json<SaveConductConfigDto>,
) -> Result<Json<ConductConfigDto>, ApiError> {
    let config = state.conduct_uc.save_config(dto.into()).await?;
    Ok(single_dto(config))
}

pub async fn get_points(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<UserConductPointsDto>, ApiError> {
    let points = state.conduct_uc.get_points(&guild_id, &user_id).await?;
    Ok(single_dto(points))
}

pub async fn get_leaderboard(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Json<Vec<UserConductPointsDto>>, ApiError> {
    let limit = normalize_limit(query.limit, 20, 50);
    let leaderboard = state.conduct_uc.get_leaderboard(&guild_id, limit).await?;
    Ok(map_to_dtos(leaderboard))
}

pub async fn get_points_log(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Query(query): Query<LogQuery>,
) -> Result<Json<Vec<ConductPointsLogDto>>, ApiError> {
    let limit = normalize_limit(query.limit, 50, 100);
    let log = state
        .conduct_uc
        .get_points_log(&guild_id, &user_id, limit)
        .await?;
    Ok(map_to_dtos(log))
}

pub async fn add_points(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AddPointsDto>,
) -> Result<Json<UserConductPointsDto>, ApiError> {
    let amount = dto.amount;
    let reason = dto.reason.clone();

    let points = state
        .conduct_uc
        .add_points(AddPointsCommand {
            guild_id: guild_id.clone(),
            user_id: user_id.clone(),
            amount,
            reason,
        })
        .await?;

    state.broadcaster.broadcast(
        "conduct_points_changed",
        serde_json::json!({
            "guild_id": &guild_id,
            "user_id": &user_id,
            "amount": amount,
            "reason": &dto.reason,
        }),
    );

    Ok(single_dto(points))
}

#[derive(serde::Serialize)]
pub struct ConductRegenTickResp {
    pub regenerated: u64,
}

/// POST /api/conduct/regen-tick
///
/// Endpoint stateless appele par le `moderation-worker` a intervalle
/// regulier. Delegue a `ManageConductUseCase::run_regen` qui applique
/// la regle metier `apply_conduct_regen` (domain). Le worker ne fait
/// que planifier — la decision vit cote API.
pub async fn run_regen_tick(
    State(state): State<AppState>,
) -> Result<Json<ConductRegenTickResp>, ApiError> {
    let regenerated = state.conduct_uc.run_regen().await?;
    Ok(Json(ConductRegenTickResp { regenerated }))
}

#[derive(serde::Serialize)]
pub struct ConductSyncBanProposalsResp {
    pub created: u64,
}

/// POST /api/conduct/sync-ban-proposals
///
/// Cree des propositions de ban (`infractions` action='ban') pour les users
/// tombes a 0 points de conduite et qui n'ont pas encore de proposition de
/// ban liee a la conduite. Idempotent. Appele periodiquement par le
/// `moderation-worker`.
pub async fn sync_ban_proposals(
    State(state): State<AppState>,
) -> Result<Json<ConductSyncBanProposalsResp>, ApiError> {
    let created = state.conduct_uc.sync_ban_proposals().await?;
    Ok(Json(ConductSyncBanProposalsResp { created }))
}

#[cfg(test)]
#[path = "tests/conduct.rs"]
mod tests;
