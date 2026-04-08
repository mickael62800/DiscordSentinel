use axum::extract::State;
use axum::Json;

use crate::adapters::inbound::http::dto::analyze::{AnalyzeRequestDto, AnalyzeResponseDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;

pub async fn analyze(
    State(state): State<AppState>,
    Json(dto): Json<AnalyzeRequestDto>,
) -> Result<Json<AnalyzeResponseDto>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;
    validation::validate_discord_id("channel_id", &dto.channel_id).map_err(ApiError)?;

    let username = dto.username.clone();
    let command = dto.into();
    let analysis = state.analyze_uc.analyze(command).await?;

    // Broadcast event if an action was taken
    let action_str = analysis.action.as_str();
    if action_str != "none" {
        state.broadcaster.broadcast(
            "infraction_new",
            serde_json::json!({
                "username": username,
                "action": action_str,
                "reason": &analysis.reason,
            }),
        );
    }

    Ok(Json(AnalyzeResponseDto::from(analysis)))
}
