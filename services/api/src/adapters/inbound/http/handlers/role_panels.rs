use axum::extract::{Path, State};
use axum::Json;

use crate::adapters::inbound::http::dto::role_panels::*;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

pub async fn create_panel(
    State(state): State<AppState>,
    Json(dto): Json<CreateRolePanelDto>,
) -> Result<Json<RolePanelDetailDto>, ApiError> {
    let detail = state.role_panels_uc.create_panel(dto.into()).await?;
    Ok(Json(RolePanelDetailDto::from(detail)))
}

pub async fn get_panel(
    State(state): State<AppState>,
    Path(panel_id): Path<String>,
) -> Result<Json<RolePanelDetailDto>, ApiError> {
    let detail = state.role_panels_uc.get_panel(&panel_id).await?;
    Ok(Json(RolePanelDetailDto::from(detail)))
}

pub async fn get_panel_by_message(
    State(state): State<AppState>,
    Path(message_id): Path<String>,
) -> Result<Json<Option<RolePanelDetailDto>>, ApiError> {
    let detail = state.role_panels_uc.get_panel_by_message(&message_id).await?;
    Ok(Json(detail.map(RolePanelDetailDto::from)))
}

pub async fn list_panels(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<RolePanelDto>>, ApiError> {
    let panels = state.role_panels_uc.list_panels(&guild_id).await?;
    Ok(Json(panels.into_iter().map(RolePanelDto::from).collect()))
}

pub async fn set_message_id(
    State(state): State<AppState>,
    Json(dto): Json<SetMessageIdDto>,
) -> Result<Json<()>, ApiError> {
    state.role_panels_uc.set_message_id(dto.into()).await?;
    Ok(Json(()))
}

pub async fn delete_panel(
    State(state): State<AppState>,
    Path(panel_id): Path<String>,
) -> Result<Json<()>, ApiError> {
    state.role_panels_uc.delete_panel(&panel_id).await?;
    Ok(Json(()))
}

pub async fn list_auto_roles(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<AutoRoleDto>>, ApiError> {
    let roles = state.role_panels_uc.list_auto_roles(&guild_id).await?;
    Ok(Json(roles.into_iter().map(AutoRoleDto::from).collect()))
}

pub async fn add_auto_role(
    State(state): State<AppState>,
    Json(dto): Json<CreateAutoRoleDto>,
) -> Result<Json<AutoRoleDto>, ApiError> {
    let role = state.role_panels_uc.add_auto_role(dto.into()).await?;
    Ok(Json(AutoRoleDto::from(role)))
}

pub async fn delete_auto_role(
    State(state): State<AppState>,
    Path((guild_id, role_id)): Path<(String, String)>,
) -> Result<Json<()>, ApiError> {
    state.role_panels_uc.delete_auto_role(&guild_id, &role_id).await?;
    Ok(Json(()))
}
