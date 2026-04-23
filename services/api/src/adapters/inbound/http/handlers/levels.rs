use axum::extract::{Path, Query, State};
use axum::Json;

use crate::adapters::inbound::http::dto::levels::{
    AddXpDto, AddXpResponseDto, LevelConfigDto, LevelLeaderboardParams, LevelRewardDto,
    SaveLevelConfigDto, SetRewardDto, UserLevelDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, normalize_limit, single_dto};
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::XpSource;
use crate::ports::inbound::manage_levels::AddXpCommand;

pub async fn get_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<LevelConfigDto>, ApiError> {
    let config = state.levels_uc.get_config(&guild_id).await?;
    Ok(single_dto(config))
}

pub async fn save_config(
    State(state): State<AppState>,
    Json(dto): Json<SaveLevelConfigDto>,
) -> Result<Json<LevelConfigDto>, ApiError> {
    let config = state.levels_uc.save_config(dto.into()).await?;
    Ok(single_dto(config))
}

pub async fn add_xp(
    State(state): State<AppState>,
    Json(dto): Json<AddXpDto>,
) -> Result<Json<AddXpResponseDto>, ApiError> {
    let guild_id = dto.guild_id.clone();
    let user_id = dto.user_id.clone();
    let amount = dto.amount;
    let source = XpSource::from_str(&dto.source);

    let result = state
        .levels_uc
        .add_xp(AddXpCommand {
            guild_id: dto.guild_id,
            user_id: dto.user_id,
            username: dto.username,
            amount: dto.amount,
            source,
        })
        .await?;

    state.broadcaster.broadcast(
        "xp_gained",
        serde_json::json!({
            "guild_id": &guild_id,
            "user_id": &user_id,
            "amount": amount,
            "source": source.as_str(),
        }),
    );

    Ok(single_dto(result))
}

pub async fn get_user_level(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<UserLevelDto>, ApiError> {
    let level = state.levels_uc.get_user_level(&guild_id, &user_id).await?;
    Ok(single_dto(level))
}

pub async fn get_leaderboard(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<LevelLeaderboardParams>,
) -> Result<Json<Vec<UserLevelDto>>, ApiError> {
    let limit = normalize_limit(params.limit, 25, 100);
    let levels = match params.source.as_deref() {
        Some("text") => state.levels_uc.get_leaderboard_by_source(&guild_id, XpSource::Text, limit).await?,
        Some("voice") => state.levels_uc.get_leaderboard_by_source(&guild_id, XpSource::Voice, limit).await?,
        _ => state.levels_uc.get_leaderboard(&guild_id, limit).await?,
    };
    Ok(map_to_dtos(levels))
}

pub async fn get_rewards(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<LevelRewardDto>>, ApiError> {
    let rewards = state.levels_uc.get_rewards(&guild_id).await?;
    Ok(map_to_dtos(rewards))
}

pub async fn set_reward(
    State(state): State<AppState>,
    Json(dto): Json<SetRewardDto>,
) -> Result<Json<LevelRewardDto>, ApiError> {
    let source = XpSource::from_str(&dto.source);
    let reward = state
        .levels_uc
        .set_reward(&dto.guild_id, dto.level, &dto.role_id, source)
        .await?;
    Ok(single_dto(reward))
}

pub async fn delete_reward(
    State(state): State<AppState>,
    Path((guild_id, level)): Path<(String, i32)>,
    Query(params): Query<DeleteRewardParams>,
) -> Result<Json<()>, ApiError> {
    let source = XpSource::from_str(params.source.as_deref().unwrap_or("text"));
    state.levels_uc.delete_reward(&guild_id, level, source).await?;
    Ok(Json(()))
}

#[derive(serde::Deserialize)]
pub struct DeleteRewardParams {
    pub source: Option<String>,
}

#[cfg(test)]
#[path = "tests/levels.rs"]
mod tests;
