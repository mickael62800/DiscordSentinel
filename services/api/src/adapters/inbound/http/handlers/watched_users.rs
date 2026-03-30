use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::watched_users::{
    UserDossierResponseDto, WatchedUserResponseDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, single_dto};
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
    Ok(map_to_dtos(users))
}

pub async fn get_user_dossier(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<UserDossierResponseDto>, ApiError> {
    let dossier = state
        .watched_users_uc
        .get_user_dossier(&guild_id, &user_id)
        .await?;
    Ok(single_dto(dossier))
}
