//! Handlers votes (motions).

use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use sentinel_core::domain::entities::influence::vote::VoteChoice;
use sentinel_core::domain::errors::DomainError;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::handlers::influence::dto::MotionStateDto;
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateMotionDto {
    pub org_name: String,
    pub creator_user_id: String,
    #[serde(default)]
    pub creator_username: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct CastVoteDto {
    pub motion_id: String,
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    pub choice: String,
}

#[derive(Debug, Deserialize)]
pub struct MotionActorDto {
    pub motion_id: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MotionIdDto {
    pub motion_id: String,
}

/// POST /api/influence/{guild_id}/motions
pub async fn create_motion(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<CreateMotionDto>,
) -> Result<Json<MotionStateDto>, ApiError> {
    let st = state
        .influence_votes_uc
        .create_motion(
            &guild_id,
            &dto.org_name,
            &dto.creator_user_id,
            &dto.creator_username,
            &dto.title,
        )
        .await?;
    Ok(Json(st.into()))
}

/// POST /api/influence/{guild_id}/motions/vote
pub async fn cast_vote(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<CastVoteDto>,
) -> Result<Json<MotionStateDto>, ApiError> {
    let choice = VoteChoice::from_str_lossy(&dto.choice).ok_or_else(|| {
        ApiError(DomainError::ValidationError(format!(
            "Choix de vote invalide : {}",
            dto.choice
        )))
    })?;
    let st = state
        .influence_votes_uc
        .cast_vote(&guild_id, &dto.motion_id, &dto.user_id, &dto.username, choice)
        .await?;
    Ok(Json(st.into()))
}

/// POST /api/influence/{guild_id}/motions/close
pub async fn close_motion(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<MotionActorDto>,
) -> Result<Json<MotionStateDto>, ApiError> {
    let st = state
        .influence_votes_uc
        .close_motion(&guild_id, &dto.motion_id, &dto.user_id)
        .await?;
    Ok(Json(st.into()))
}

/// POST /api/influence/{guild_id}/motions/state
pub async fn motion_state(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<MotionIdDto>,
) -> Result<Json<MotionStateDto>, ApiError> {
    let st = state
        .influence_votes_uc
        .get_state(&guild_id, &dto.motion_id)
        .await?;
    Ok(Json(st.into()))
}
