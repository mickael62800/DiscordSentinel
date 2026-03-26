use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::adapters::inbound::http::dto::stats::{
    GuildOverviewDto, LeaderboardQuery, RecordMessagesDto, RecordVoiceDto, UserStatsDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

/// POST /api/stats/messages — enregistrer des messages
pub async fn record_messages(
    State(state): State<AppState>,
    Json(dto): Json<RecordMessagesDto>,
) -> Result<StatusCode, ApiError> {
    state.stats_uc.record_messages(dto.into()).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/stats/voice — enregistrer du temps vocal
pub async fn record_voice(
    State(state): State<AppState>,
    Json(dto): Json<RecordVoiceDto>,
) -> Result<StatusCode, ApiError> {
    state.stats_uc.record_voice(dto.into()).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/stats/{guild_id}/user/{user_id} — stats d'un utilisateur
pub async fn get_user_stats(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Option<UserStatsDto>>, ApiError> {
    let stats = state.stats_uc.get_user_stats(&guild_id, &user_id).await?;
    Ok(Json(stats.map(UserStatsDto::from)))
}

/// GET /api/stats/{guild_id}/overview — stats globales du serveur
pub async fn get_guild_overview(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<GuildOverviewDto>, ApiError> {
    let overview = state.stats_uc.get_guild_overview(&guild_id).await?;
    Ok(Json(GuildOverviewDto::from(overview)))
}

/// GET /api/stats/{guild_id}/leaderboard — classement
pub async fn get_leaderboard(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<LeaderboardQuery>,
) -> Result<Json<Vec<UserStatsDto>>, ApiError> {
    let limit = params.limit.unwrap_or(10).min(50);
    let members = state.stats_uc.get_leaderboard(&guild_id, limit).await?;
    Ok(Json(members.into_iter().map(UserStatsDto::from).collect()))
}
