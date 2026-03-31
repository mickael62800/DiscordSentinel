use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::conduct::{
    AddPointsDto, ConductConfigDto, ConductPointsLogDto, SaveConductConfigDto,
    UserConductPointsDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, normalize_limit, single_dto};
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
