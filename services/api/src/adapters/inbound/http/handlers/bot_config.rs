use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use redis::AsyncCommands;

use crate::adapters::inbound::http::dto::bot_config::{
    BotDefinitionDto, BotGuildConfigDto, DeleteConfigDto, SetConfigDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::state::AppState;

const DEFINITIONS_TTL: u64 = 3600; // 1 heure
const GUILD_CONFIG_TTL: u64 = 900; // 15 minutes

/// GET /api/bots/definitions — liste des bots et leurs parametres disponibles (cache 1h)
pub async fn get_definitions(
    State(state): State<AppState>,
) -> Result<Json<Vec<BotDefinitionDto>>, ApiError> {
    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(cached) = conn.get::<_, Option<String>>("bot:definitions").await {
            if let Some(json) = cached {
                if let Ok(dtos) = serde_json::from_str::<Vec<BotDefinitionDto>>(&json) {
                    return Ok(Json(dtos));
                }
            }
        }
    }

    let defs = state.bot_config_repo.get_definitions().await?;
    let dtos: Vec<BotDefinitionDto> = defs.into_iter().map(BotDefinitionDto::from).collect();

    // Populate cache
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&dtos) {
            let _: Result<(), _> = conn.set_ex("bot:definitions", json, DEFINITIONS_TTL).await;
        }
    }

    Ok(Json(dtos))
}

/// GET /api/bots/config/{guild_id} — config de tous les bots pour un serveur (cache 15min)
pub async fn get_guild_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<BotGuildConfigDto>>, ApiError> {
    let cache_key = format!("bot:config:{guild_id}");

    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(cached) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Some(json) = cached {
                if let Ok(dtos) = serde_json::from_str::<Vec<BotGuildConfigDto>>(&json) {
                    return Ok(Json(dtos));
                }
            }
        }
    }

    let configs = state.bot_config_repo.get_all_config(&guild_id).await?;
    let dtos: Vec<BotGuildConfigDto> = configs.into_iter().map(BotGuildConfigDto::from).collect();

    // Populate cache
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&dtos) {
            let _: Result<(), _> = conn.set_ex(&cache_key, json, GUILD_CONFIG_TTL).await;
        }
    }

    Ok(Json(dtos))
}

/// GET /api/bots/config/{guild_id}/{bot_name} — config d'un bot specifique pour un serveur
pub async fn get_bot_config(
    State(state): State<AppState>,
    Path((guild_id, bot_name)): Path<(String, String)>,
) -> Result<Json<Vec<BotGuildConfigDto>>, ApiError> {
    let cache_key = format!("bot:config:{guild_id}:{bot_name}");

    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(cached) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Some(json) = cached {
                if let Ok(dtos) = serde_json::from_str::<Vec<BotGuildConfigDto>>(&json) {
                    return Ok(Json(dtos));
                }
            }
        }
    }

    let configs = state.bot_config_repo.get_config(&guild_id, &bot_name).await?;
    let dtos: Vec<BotGuildConfigDto> = configs.into_iter().map(BotGuildConfigDto::from).collect();

    // Populate cache
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&dtos) {
            let _: Result<(), _> = conn.set_ex(&cache_key, json, GUILD_CONFIG_TTL).await;
        }
    }

    Ok(Json(dtos))
}

/// POST /api/bots/config — sauvegarder un parametre + invalider le cache
pub async fn set_config(
    State(state): State<AppState>,
    Json(dto): Json<SetConfigDto>,
) -> Result<StatusCode, ApiError> {
    state
        .bot_config_repo
        .set_config(&dto.guild_id, &dto.bot_name, &dto.config_key, &dto.config_value)
        .await?;

    // Invalider les caches
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let _: Result<(), _> = conn.del(format!("bot:config:{}", dto.guild_id)).await;
        let _: Result<(), _> = conn.del(format!("bot:config:{}:{}", dto.guild_id, dto.bot_name)).await;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/bots/config — supprimer un parametre + invalider le cache
pub async fn delete_config(
    State(state): State<AppState>,
    Json(dto): Json<DeleteConfigDto>,
) -> Result<StatusCode, ApiError> {
    state
        .bot_config_repo
        .delete_config(&dto.guild_id, &dto.bot_name, &dto.config_key)
        .await?;

    // Invalider les caches
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let _: Result<(), _> = conn.del(format!("bot:config:{}", dto.guild_id)).await;
        let _: Result<(), _> = conn.del(format!("bot:config:{}:{}", dto.guild_id, dto.bot_name)).await;
    }

    Ok(StatusCode::NO_CONTENT)
}
