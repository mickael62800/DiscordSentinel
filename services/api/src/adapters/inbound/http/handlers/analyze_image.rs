use axum::extract::State;
use axum::Json;
use base64::Engine;

use crate::adapters::inbound::http::dto::analyze_image::{AnalyzeImageRequestDto, AnalyzeImageResponseDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;
use crate::ports::inbound::AnalyzeImageCommand;

pub async fn analyze_image(
    State(state): State<AppState>,
    Json(dto): Json<AnalyzeImageRequestDto>,
) -> Result<Json<AnalyzeImageResponseDto>, ApiError> {
    // Decoder le base64
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(&dto.image_data)
        .map_err(|e| ApiError(DomainError::ValidationError(format!("Base64 invalide: {e}"))))?;

    let username = dto.username.clone();

    let command = AnalyzeImageCommand {
        guild_id: dto.guild_id,
        channel_id: dto.channel_id,
        user_id: dto.user_id,
        username: dto.username,
        message_id: dto.message_id,
        image_bytes,
        content_type: dto.content_type,
        filename: dto.filename,
    };

    let analysis = state.analyze_image_uc.analyze_image(command).await?;

    // Broadcast si action prise
    let action_str = analysis.action.as_str();
    if action_str != "none" {
        state.broadcaster.broadcast(
            "infraction_new",
            serde_json::json!({
                "username": username,
                "action": action_str,
                "reason": &analysis.reason,
                "type": "image",
            }),
        );
    }

    Ok(Json(AnalyzeImageResponseDto::from(analysis)))
}
