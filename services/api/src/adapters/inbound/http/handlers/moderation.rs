use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::moderation::{
    BanEntryDto, LogActionDto, ModerationActionResponseDto, UserHistoryDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, ok_response, single_dto};
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Deserialize)]
pub struct BansQuery {
    pub guild_id: Option<String>,
}

/// POST /api/moderation/actions — enregistrer une action de modération
pub async fn log_action(
    State(state): State<AppState>,
    Json(dto): Json<LogActionDto>,
) -> Result<Json<ModerationActionResponseDto>, ApiError> {
    let action_type = dto.action_type.clone();
    let target_name = dto.target_name.clone();
    let moderator_name = dto.moderator_name.clone();
    let reason = dto.reason.clone();

    let command = dto.into();
    let action = state.moderation_uc.log_action(command).await?;

    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": action_type,
            "target_name": target_name,
            "moderator_name": moderator_name,
            "reason": reason,
        }),
    );

    Ok(single_dto(action))
}

#[derive(Debug, Deserialize)]
pub struct ExecuteBanDto {
    pub guild_id: String,
    pub user_id: String,
    pub reason: String,
}

/// POST /api/moderation/execute-ban — execute un ban Discord + log l'action
pub async fn execute_ban(
    State(state): State<AppState>,
    Json(dto): Json<ExecuteBanDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .discord_api
        .ban_user(&dto.guild_id, &dto.user_id, &dto.reason)
        .await
        .map_err(ApiError)?;

    let command = crate::ports::inbound::LogModerationCommand {
        guild_id: dto.guild_id.clone(),
        channel_id: String::new(),
        moderator_id: "desktop".into(),
        moderator_name: "Desktop App".into(),
        target_id: dto.user_id.clone(),
        target_name: dto.user_id.clone(),
        action_type: "ban_permanent".into(),
        reason: dto.reason,
        gravity: None,
        duration: None,
    };
    state.moderation_uc.log_action(command).await?;

    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct ExecuteUnbanDto {
    pub guild_id: String,
    pub user_id: String,
}

/// POST /api/moderation/execute-unban — debannir un utilisateur Discord
pub async fn execute_unban(
    State(state): State<AppState>,
    Json(dto): Json<ExecuteUnbanDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .discord_api
        .unban_user(&dto.guild_id, &dto.user_id)
        .await
        .map_err(ApiError)?;

    let command = crate::ports::inbound::LogModerationCommand {
        guild_id: dto.guild_id.clone(),
        channel_id: String::new(),
        moderator_id: "desktop".into(),
        moderator_name: "Desktop App".into(),
        target_id: dto.user_id.clone(),
        target_name: dto.user_id,
        action_type: "unban".into(),
        reason: "Deban depuis le desktop".into(),
        gravity: None,
        duration: None,
    };
    state
        .moderation_uc
        .delete_bans_for_user(&dto.guild_id, &command.target_id)
        .await?;
    state.moderation_uc.log_action(command).await?;

    Ok(ok_response())
}

/// GET /api/moderation/bans
pub async fn list_bans(
    State(state): State<AppState>,
    Query(params): Query<BansQuery>,
) -> Result<Json<Vec<BanEntryDto>>, ApiError> {
    let bans = state
        .moderation_uc
        .list_bans(params.guild_id.as_deref())
        .await?;
    Ok(map_to_dtos(bans))
}

/// GET /api/moderation/history/{guild_id}/{user_id}
pub async fn get_history(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<UserHistoryDto>, ApiError> {
    let history = state
        .moderation_uc
        .get_history(&guild_id, &user_id)
        .await?;
    Ok(single_dto(history))
}
