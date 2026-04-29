use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::audit::user_activity::UserActivity;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Deserialize)]
pub struct CreateActivityDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub event_type: String,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub content: Option<String>,
    #[serde(default = "default_metadata")]
    pub metadata: serde_json::Value,
}

fn default_metadata() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Debug, Deserialize)]
pub struct ActivityQuery {
    pub event_type: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// POST /api/user-activity — enregistrer un evenement d'activite.
/// Passe par le repository (plus de SQL direct dans le handler).
pub async fn create_activity(
    State(state): State<AppState>,
    Json(dto): Json<CreateActivityDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let activity = UserActivity {
        id: uuid::Uuid::new_v4(),
        guild_id: dto.guild_id,
        user_id: dto.user_id,
        event_type: dto.event_type,
        channel_id: dto.channel_id,
        channel_name: dto.channel_name,
        content: dto.content,
        metadata: dto.metadata,
        created_at: chrono::Utc::now(),
    };

    state.user_activity_repo.create(&activity).await?;
    Ok(ok_response())
}

/// GET /api/user-activity/{guild_id}/{user_id} — timeline d'un utilisateur
pub async fn get_activity(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Query(params): Query<ActivityQuery>,
) -> Result<Json<Vec<UserActivity>>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(200) as i64;
    let offset = params.offset.unwrap_or(0) as i64;

    let activities = state
        .user_activity_repo
        .list(&guild_id, &user_id, params.event_type.as_deref(), limit, offset)
        .await?;
    Ok(Json(activities))
}

#[cfg(test)]
#[path = "tests/user_activity.rs"]
mod tests;
