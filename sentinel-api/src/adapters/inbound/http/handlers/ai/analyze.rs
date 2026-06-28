use axum::extract::State;
use axum::Json;

use crate::adapters::inbound::http::dto::ai::analyze::AnalyzeRequestDto;
use crate::adapters::inbound::http::dto::ai::analyze::AnalyzeResponseDto;
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

    let (command, (username, guild_id)) = crate::capture_and_into!(dto, username, guild_id);
    let analysis = state.analyze_uc.analyze(command).await?;

    // Broadcast event if an action was taken.
    // M7 — comparaison typee (enum) au lieu de string pour eviter la
    // divergence silencieuse si as_str() change un jour.
    if analysis.action != sentinel_core::domain::enums::moderation::action::Action::None {
        state.broadcaster.broadcast(
            "infraction_new",
            serde_json::json!({
                "guild_id": guild_id,
                "username": username,
                "action": analysis.action.as_str(),
                "reason": &analysis.reason,
            }),
        );
    }

    Ok(Json(AnalyzeResponseDto::from(analysis)))
}
