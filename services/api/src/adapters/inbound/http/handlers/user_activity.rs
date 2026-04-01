use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::UserActivity;

#[derive(Debug, Deserialize)]
pub struct CreateActivityDto {
    pub guild_id: String,
    pub user_id: String,
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

/// POST /api/user-activity — enregistrer un evenement d'activite
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

    sqlx::query(
        r#"
        INSERT INTO user_activity_log (id, guild_id, user_id, event_type, channel_id, channel_name, content, metadata, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(activity.id)
    .bind(&activity.guild_id)
    .bind(&activity.user_id)
    .bind(&activity.event_type)
    .bind(&activity.channel_id)
    .bind(&activity.channel_name)
    .bind(&activity.content)
    .bind(&activity.metadata)
    .bind(activity.created_at)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| crate::domain::errors::DomainError::Internal(e.to_string()))?;

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

    let rows = if let Some(ref event_type) = params.event_type {
        sqlx::query_as::<_, ActivityRow>(
            r#"
            SELECT id, guild_id, user_id, event_type, channel_id, channel_name, content, metadata, created_at
            FROM user_activity_log
            WHERE guild_id = $1 AND user_id = $2 AND event_type = $3
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(&guild_id)
        .bind(&user_id)
        .bind(event_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pg_pool)
        .await
    } else {
        sqlx::query_as::<_, ActivityRow>(
            r#"
            SELECT id, guild_id, user_id, event_type, channel_id, channel_name, content, metadata, created_at
            FROM user_activity_log
            WHERE guild_id = $1 AND user_id = $2
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(&guild_id)
        .bind(&user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pg_pool)
        .await
    }
    .map_err(|e| crate::domain::errors::DomainError::Internal(e.to_string()))?;

    let activities: Vec<UserActivity> = rows.into_iter().map(|r| r.into()).collect();
    Ok(Json(activities))
}

#[derive(sqlx::FromRow)]
struct ActivityRow {
    id: uuid::Uuid,
    guild_id: String,
    user_id: String,
    event_type: String,
    channel_id: Option<String>,
    channel_name: Option<String>,
    content: Option<String>,
    metadata: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ActivityRow> for UserActivity {
    fn from(r: ActivityRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            user_id: r.user_id,
            event_type: r.event_type,
            channel_id: r.channel_id,
            channel_name: r.channel_name,
            content: r.content,
            metadata: r.metadata,
            created_at: r.created_at,
        }
    }
}
