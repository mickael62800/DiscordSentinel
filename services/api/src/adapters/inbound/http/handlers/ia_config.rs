use axum::extract::{Path, State};
use axum::Json;

use crate::adapters::inbound::http::dto::ia_config::{IaConfigDto, SaveIaConfigDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::IaConfig;

/// GET /api/ia-config/{guild_id}
pub async fn get_ia_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<IaConfigDto>, ApiError> {
    let config = state
        .ia_config_repo
        .get(&guild_id)
        .await?
        .unwrap_or_else(|| IaConfig::default_for_guild(&guild_id));

    Ok(Json(IaConfigDto::from(config)))
}

/// PUT /api/ia-config/{guild_id}
pub async fn save_ia_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<SaveIaConfigDto>,
) -> Result<Json<IaConfigDto>, ApiError> {
    // La normalisation (clamp thresholds/max_*, fallback context_format) est
    // une regle metier : voir `IaConfig::new_normalized` dans domain/entities.
    let config = IaConfig::new_normalized(
        guild_id,
        dto.text_enabled,
        dto.text_threshold,
        dto.vision_enabled,
        dto.vision_threshold,
        dto.context_dampening,
        dto.context_format,
        dto.context_max_messages,
        dto.context_max_chars,
    );

    let saved = state.ia_config_repo.save(&config).await?;
    Ok(Json(IaConfigDto::from(saved)))
}
