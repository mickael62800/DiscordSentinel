use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use serde::Deserialize;

use crate::adapters::inbound::http::dto::watched_users::{
    UserDossierResponseDto, WatchedUserResponseDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, ok_response, single_dto};
use crate::adapters::inbound::http::middleware::rbac::{check_role, Role, RoleContext};
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Deserialize)]
pub struct WatchedUsersQueryParams {
    pub guild_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_watched_users(
    State(state): State<AppState>,
    Query(params): Query<WatchedUsersQueryParams>,
) -> Result<Json<Vec<WatchedUserResponseDto>>, ApiError> {
    let limit = crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 200);
    let offset = params.offset.unwrap_or(0).max(0);
    let users = state
        .watched_users_uc
        .list_watched_users(params.guild_id.as_deref(), limit, offset)
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

#[derive(Debug, Deserialize)]
pub struct AddWatchDto {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    #[serde(default)]
    pub reason: String,
}

/// POST /api/watched-users — ajouter un utilisateur en surveillance manuelle
pub async fn add_watched_user(
    State(state): State<AppState>,
    Json(dto): Json<AddWatchDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .watched_users_uc
        .add_manual_watch(&dto.guild_id, &dto.user_id, &dto.username, &dto.reason)
        .await?;

    state.broadcaster.broadcast(
        "watched_user_added",
        serde_json::json!({
            "guild_id": &dto.guild_id,
            "user_id": &dto.user_id,
            "username": &dto.username,
        }),
    );

    Ok(ok_response())
}

/// DELETE /api/watched-users/{guild_id}/{user_id} — retirer de la surveillance manuelle
pub async fn remove_watched_user(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_role(
        &rbac,
        Role::Moderator,
        "moderator+ requis pour retirer un watched user",
    )?;
    state
        .watched_users_uc
        .remove_manual_watch(&guild_id, &user_id)
        .await?;

    Ok(ok_response())
}
