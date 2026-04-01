use axum::extract::{Path, State};
use axum::Json;
use redis::AsyncCommands;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::services::DiscordMember;

const MEMBERS_TTL: u64 = 600; // 10 minutes

/// GET /api/guilds/{guild_id}/members — liste les membres Discord (cache 10min)
pub async fn list_members(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<DiscordMember>>, ApiError> {
    let cache_key = format!("guild:members:{guild_id}");

    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(cached) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Some(json) = cached {
                if let Ok(members) = serde_json::from_str::<Vec<DiscordMember>>(&json) {
                    return Ok(Json(members));
                }
            }
        }
    }

    let members = state.discord_api.list_members(&guild_id, 1000).await?;

    // Populate cache
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&members) {
            let _: Result<(), _> = conn.set_ex(&cache_key, json, MEMBERS_TTL).await;
        }
    }

    Ok(Json(members))
}
