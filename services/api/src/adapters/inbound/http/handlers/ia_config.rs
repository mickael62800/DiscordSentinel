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
    let config = IaConfig {
        guild_id: guild_id.clone(),
        text_enabled: dto.text_enabled,
        text_threshold: dto.text_threshold.clamp(0.0, 1.0),
        vision_enabled: dto.vision_enabled,
        vision_threshold: dto.vision_threshold.clamp(0.0, 1.0),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let saved = state.ia_config_repo.save(&config).await?;
    Ok(Json(IaConfigDto::from(saved)))
}
