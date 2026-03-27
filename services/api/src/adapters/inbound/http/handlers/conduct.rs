use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::conduct::{
    AddPointsDto, ConductConfigDto, ConductPointsLogDto, SaveConductConfigDto,
    UserConductPointsDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::AddPointsCommand;

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
    Ok(Json(ConductConfigDto::from(config)))
}

pub async fn save_config(
    State(state): State<AppState>,
    Json(dto): Json<SaveConductConfigDto>,
) -> Result<Json<ConductConfigDto>, ApiError> {
    let config = state.conduct_uc.save_config(dto.into()).await?;
    Ok(Json(ConductConfigDto::from(config)))
}

pub async fn get_points(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<UserConductPointsDto>, ApiError> {
    let points = state.conduct_uc.get_points(&guild_id, &user_id).await?;
    Ok(Json(UserConductPointsDto::from(points)))
}

pub async fn get_leaderboard(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Json<Vec<UserConductPointsDto>>, ApiError> {
    let limit = query.limit.unwrap_or(20).min(50);
    let leaderboard = state.conduct_uc.get_leaderboard(&guild_id, limit).await?;
    let dtos: Vec<UserConductPointsDto> = leaderboard
        .into_iter()
        .map(UserConductPointsDto::from)
        .collect();
    Ok(Json(dtos))
}

pub async fn get_points_log(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Query(query): Query<LogQuery>,
) -> Result<Json<Vec<ConductPointsLogDto>>, ApiError> {
    let limit = query.limit.unwrap_or(50).min(100);
    let log = state
        .conduct_uc
        .get_points_log(&guild_id, &user_id, limit)
        .await?;
    let dtos: Vec<ConductPointsLogDto> = log.into_iter().map(ConductPointsLogDto::from).collect();
    Ok(Json(dtos))
}

pub async fn add_points(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AddPointsDto>,
) -> Result<Json<UserConductPointsDto>, ApiError> {
    let points = state
        .conduct_uc
        .add_points(AddPointsCommand {
            guild_id,
            user_id,
            amount: dto.amount,
            reason: dto.reason,
        })
        .await?;
    Ok(Json(UserConductPointsDto::from(points)))
}
