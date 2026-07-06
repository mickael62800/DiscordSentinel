//! Endpoint GET /api/games/servers/{server_id}/sessions — historique
//! des sessions joueurs (joined_at / left_at / duration_seconds).

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::entities::game::player_session::PlayerSession;

#[derive(Debug, Deserialize)]
pub struct SessionsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PlayerSessionDto {
    pub id: Uuid,
    pub server_id: Uuid,
    pub player_name: String,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
}

impl From<PlayerSession> for PlayerSessionDto {
    fn from(s: PlayerSession) -> Self {
        Self {
            id: s.id,
            server_id: s.server_id,
            player_name: s.player_name,
            joined_at: s.joined_at,
            left_at: s.left_at,
            duration_seconds: s.duration_seconds,
        }
    }
}

pub async fn list_sessions(
    State(state): State<AppState>,
    rbac: Option<axum::Extension<crate::adapters::inbound::http::middleware::rbac::RoleContext>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<SessionsQuery>,
) -> Result<Json<Vec<PlayerSessionDto>>, ApiError> {
    super::servers::gate_server(
        &state,
        &rbac,
        server_id,
        "game.session.view",
        "role insuffisant pour consulter les sessions",
    )
    .await?;
    let limit = q.limit.unwrap_or(100).clamp(1, 1000);
    let offset = q.offset.unwrap_or(0).max(0);
    let list = state
        .game_session_repo
        .list_history(server_id, limit, offset)
        .await?;
    Ok(Json(list.into_iter().map(PlayerSessionDto::from).collect()))
}
