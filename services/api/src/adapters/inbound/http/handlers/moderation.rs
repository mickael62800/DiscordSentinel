use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::moderation::{
    BanEntryDto, LogActionDto, ModerationActionResponseDto, UserHistoryDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

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

    // Broadcast WebSocket
    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": action_type,
            "target_name": target_name,
            "moderator_name": moderator_name,
            "reason": reason,
        }),
    );

    Ok(Json(ModerationActionResponseDto::from(action)))
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
    if state.discord_bot_token.is_empty() {
        return Err(ApiError(DomainError::Internal("MODERATION_DISCORD_TOKEN non configure".into())));
    }

    // Appeler l'API Discord pour bannir
    let client = reqwest::Client::new();
    let url = format!(
        "https://discord.com/api/v10/guilds/{}/bans/{}",
        dto.guild_id, dto.user_id
    );

    let resp = client
        .put(&url)
        .header("Authorization", format!("Bot {}", state.discord_bot_token))
        .json(&serde_json::json!({
            "delete_message_seconds": 86400,
            "reason": dto.reason,
        }))
        .send()
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("Discord API error: {e}"))))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(ApiError(DomainError::Internal(format!("Discord ban failed ({status}): {body}"))));
    }

    // Log l'action de moderation
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

    Ok(Json(serde_json::json!({ "success": true })))
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
    if state.discord_bot_token.is_empty() {
        return Err(ApiError(DomainError::Internal("MODERATION_DISCORD_TOKEN non configure".into())));
    }

    let client = reqwest::Client::new();
    let url = format!(
        "https://discord.com/api/v10/guilds/{}/bans/{}",
        dto.guild_id, dto.user_id
    );

    let resp = client
        .delete(&url)
        .header("Authorization", format!("Bot {}", state.discord_bot_token))
        .send()
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("Discord API error: {e}"))))?;

    let status = resp.status();
    if !status.is_success() && status.as_u16() != 404 {
        let body = resp.text().await.unwrap_or_default();
        return Err(ApiError(DomainError::Internal(format!("Discord unban failed ({status}): {body}"))));
    }

    // Log l'action
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
    // Supprimer les entrees ban de moderation_actions
    state.moderation_uc.delete_bans_for_user(&dto.guild_id, &command.target_id).await?;

    // Log le unban
    state.moderation_uc.log_action(command).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// GET /api/moderation/bans
pub async fn list_bans(
    State(state): State<AppState>,
    Query(params): Query<BansQuery>,
) -> Result<Json<Vec<BanEntryDto>>, ApiError> {
    let bans = state.moderation_uc.list_bans(params.guild_id.as_deref()).await?;
    Ok(Json(bans.into_iter().map(BanEntryDto::from).collect()))
}

/// GET /api/moderation/history/{guild_id}/{user_id}
pub async fn get_history(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<UserHistoryDto>, ApiError> {
    let history = state.moderation_uc.get_history(&guild_id, &user_id).await?;
    Ok(Json(UserHistoryDto::from(history)))
}
