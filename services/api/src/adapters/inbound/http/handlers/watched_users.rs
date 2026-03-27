use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::watched_users::{
    UserDossierResponseDto, WatchedUserResponseDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Deserialize)]
pub struct WatchedUsersQueryParams {
    pub guild_id: Option<String>,
}

pub async fn list_watched_users(
    State(state): State<AppState>,
    Query(params): Query<WatchedUsersQueryParams>,
) -> Result<Json<Vec<WatchedUserResponseDto>>, ApiError> {
    let users = state
        .watched_users_uc
        .list_watched_users(params.guild_id.as_deref())
        .await?;
    let dtos: Vec<WatchedUserResponseDto> = users
        .into_iter()
        .map(WatchedUserResponseDto::from)
        .collect();
    Ok(Json(dtos))
}

pub async fn get_user_dossier(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<UserDossierResponseDto>, ApiError> {
    let dossier = state
        .watched_users_uc
        .get_user_dossier(&guild_id, &user_id)
        .await?;
    Ok(Json(UserDossierResponseDto::from(dossier)))
}
