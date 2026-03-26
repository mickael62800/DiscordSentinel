use axum::extract::{Path, State};
use axum::Json;

use crate::adapters::inbound::http::dto::moderation::{
    LogActionDto, ModerationActionResponseDto, UserHistoryDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

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

/// GET /api/moderation/history/{guild_id}/{user_id}
pub async fn get_history(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<UserHistoryDto>, ApiError> {
    let history = state.moderation_uc.get_history(&guild_id, &user_id).await?;
    Ok(Json(UserHistoryDto::from(history)))
}
