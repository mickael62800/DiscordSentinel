use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::adapters::inbound::http::dto::bot_config::{
    BotDefinitionDto, BotGuildConfigDto, DeleteConfigDto, SetConfigDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::state::AppState;

/// GET /api/bots/definitions — liste des bots et leurs parametres disponibles
pub async fn get_definitions(
    State(state): State<AppState>,
) -> Result<Json<Vec<BotDefinitionDto>>, ApiError> {
    let defs = state.bot_config_repo.get_definitions().await?;
    Ok(map_to_dtos(defs))
}

/// GET /api/bots/config/{guild_id} — config de tous les bots pour un serveur
pub async fn get_guild_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<BotGuildConfigDto>>, ApiError> {
    let configs = state.bot_config_repo.get_all_config(&guild_id).await?;
    Ok(map_to_dtos(configs))
}

/// GET /api/bots/config/{guild_id}/{bot_name} — config d'un bot specifique pour un serveur
pub async fn get_bot_config(
    State(state): State<AppState>,
    Path((guild_id, bot_name)): Path<(String, String)>,
) -> Result<Json<Vec<BotGuildConfigDto>>, ApiError> {
    let configs = state.bot_config_repo.get_config(&guild_id, &bot_name).await?;
    Ok(map_to_dtos(configs))
}

/// POST /api/bots/config — sauvegarder un parametre
pub async fn set_config(
    State(state): State<AppState>,
    Json(dto): Json<SetConfigDto>,
) -> Result<StatusCode, ApiError> {
    state
        .bot_config_repo
        .set_config(&dto.guild_id, &dto.bot_name, &dto.config_key, &dto.config_value)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/bots/config — supprimer un parametre
pub async fn delete_config(
    State(state): State<AppState>,
    Json(dto): Json<DeleteConfigDto>,
) -> Result<StatusCode, ApiError> {
    state
        .bot_config_repo
        .delete_config(&dto.guild_id, &dto.bot_name, &dto.config_key)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
