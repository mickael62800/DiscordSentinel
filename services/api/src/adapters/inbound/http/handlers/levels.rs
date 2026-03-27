use axum::extract::{Path, Query, State};
use axum::Json;

use crate::adapters::inbound::http::dto::levels::{
    AddXpDto, AddXpResponseDto, LevelConfigDto, LevelLeaderboardParams, LevelRewardDto,
    SaveLevelConfigDto, SetRewardDto, UserLevelDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::manage_levels::AddXpCommand;

pub async fn get_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<LevelConfigDto>, ApiError> {
    let config = state.levels_uc.get_config(&guild_id).await?;
    Ok(Json(LevelConfigDto::from(config)))
}

pub async fn save_config(
    State(state): State<AppState>,
    Json(dto): Json<SaveLevelConfigDto>,
) -> Result<Json<LevelConfigDto>, ApiError> {
    let config = state.levels_uc.save_config(dto.into()).await?;
    Ok(Json(LevelConfigDto::from(config)))
}

pub async fn add_xp(
    State(state): State<AppState>,
    Json(dto): Json<AddXpDto>,
) -> Result<Json<AddXpResponseDto>, ApiError> {
    let result = state
        .levels_uc
        .add_xp(AddXpCommand {
            guild_id: dto.guild_id,
            user_id: dto.user_id,
            username: dto.username,
            amount: dto.amount,
        })
        .await?;
    Ok(Json(AddXpResponseDto::from(result)))
}

pub async fn get_user_level(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<UserLevelDto>, ApiError> {
    let level = state.levels_uc.get_user_level(&guild_id, &user_id).await?;
    Ok(Json(UserLevelDto::from(level)))
}

pub async fn get_leaderboard(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<LevelLeaderboardParams>,
) -> Result<Json<Vec<UserLevelDto>>, ApiError> {
    let limit = params.limit.unwrap_or(25).min(100);
    let levels = state.levels_uc.get_leaderboard(&guild_id, limit).await?;
    let dtos: Vec<UserLevelDto> = levels.into_iter().map(UserLevelDto::from).collect();
    Ok(Json(dtos))
}

pub async fn get_rewards(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<LevelRewardDto>>, ApiError> {
    let rewards = state.levels_uc.get_rewards(&guild_id).await?;
    let dtos: Vec<LevelRewardDto> = rewards.into_iter().map(LevelRewardDto::from).collect();
    Ok(Json(dtos))
}

pub async fn set_reward(
    State(state): State<AppState>,
    Json(dto): Json<SetRewardDto>,
) -> Result<Json<LevelRewardDto>, ApiError> {
    let reward = state
        .levels_uc
        .set_reward(&dto.guild_id, dto.level, &dto.role_id)
        .await?;
    Ok(Json(LevelRewardDto::from(reward)))
}

pub async fn delete_reward(
    State(state): State<AppState>,
    Path((guild_id, level)): Path<(String, i32)>,
) -> Result<Json<()>, ApiError> {
    state.levels_uc.delete_reward(&guild_id, level).await?;
    Ok(Json(()))
}
