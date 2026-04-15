//! Handler HTTP pour lister les salons texte d'une guild (Phase 9 Part E).
//!
//! Utilise par les pages web admin qui ont besoin d'un channel picker
//! (config railleries, config salons d'activite, etc.). Cache Redis 10min
//! pour eviter de taper Discord a chaque ouverture de page.

use axum::extract::{Path, State};
use axum::Json;
use redis::AsyncCommands;
use tracing::warn;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::services::DiscordChannel;

const CHANNELS_TTL: u64 = 600;

/// GET /api/guilds/{guild_id}/channels — liste les salons texte.
pub async fn list_text_channels(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<DiscordChannel>>, ApiError> {
    let cache_key = format!("guild:channels:{guild_id}");

    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(json)) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Ok(channels) = serde_json::from_str::<Vec<DiscordChannel>>(&json) {
                return Ok(Json(channels));
            }
        }
    }

    let channels = state.discord_api.list_text_channels(&guild_id).await?;

    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&channels) {
            if let Err(e) = conn.set_ex::<_, _, ()>(&cache_key, json, CHANNELS_TTL).await {
                warn!(error = %e, cache_key = %cache_key, "Echec cache set channels");
            }
        }
    }

    Ok(Json(channels))
}
