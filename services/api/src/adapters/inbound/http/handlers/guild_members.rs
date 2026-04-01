use axum::extract::{Path, State};
use axum::Json;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::services::DiscordMember;

/// GET /api/guilds/{guild_id}/members — liste les membres Discord d'un serveur
pub async fn list_members(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<DiscordMember>>, ApiError> {
    let members = state.discord_api.list_members(&guild_id, 1000).await?;
    Ok(Json(members))
}
