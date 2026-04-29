//! Handler HTTP /travaux (Phase 2 #2 audit).

use axum::extract::Path;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::coude::play_travaux::PlayTravauxCommand;
use crate::ports::inbound::coude::play_travaux::TravauxResolution;
use crate::domain::entities::system::discord_ids::UserId;
#[derive(Debug, Deserialize)]
pub struct PlayTravauxDto {
    pub user_id: UserId,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct TravauxResolutionDto {
    pub task_key: String,
    pub task_label: String,
    pub task_description: String,
    pub success: bool,
    pub flavor: String,
    pub coins_gain: i64,
    pub xp_gain: i64,
}

impl From<TravauxResolution> for TravauxResolutionDto {
    fn from(r: TravauxResolution) -> Self {
        Self {
            task_key: r.task_key.into(),
            task_label: r.task_label.into(),
            task_description: r.task_description.into(),
            success: r.success,
            flavor: r.flavor.into(),
            coins_gain: r.coins_gain,
            xp_gain: r.xp_gain,
        }
    }
}

/// POST /api/coude/{guild_id}/travaux/play
pub async fn play_travaux(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<PlayTravauxDto>,
) -> Result<Json<TravauxResolutionDto>, ApiError> {
    let res = state
        .play_travaux_uc
        .play(PlayTravauxCommand {
            guild_id: guild_id.into(),
            user_id: dto.user_id,
            username: dto.username,
        })
        .await?;
    Ok(Json(res.into()))
}
