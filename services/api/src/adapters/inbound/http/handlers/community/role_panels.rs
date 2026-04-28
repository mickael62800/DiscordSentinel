use axum::extract::{Path, State};
use axum::{Extension, Json};

use crate::adapters::inbound::http::dto::role_panels::*;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, single_dto};
use crate::adapters::inbound::http::middleware::rbac::{
    check_role_for_guild, require_role, Role, RoleContext,
};
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

pub async fn create_panel(
    State(state): State<AppState>,
    Json(dto): Json<CreateRolePanelDto>,
) -> Result<Json<RolePanelDetailDto>, ApiError> {
    let detail = state.role_panels_uc.create_panel(dto.into()).await?;
    Ok(single_dto(detail))
}

pub async fn get_panel(
    State(state): State<AppState>,
    Path(panel_id): Path<String>,
) -> Result<Json<RolePanelDetailDto>, ApiError> {
    let detail = state.role_panels_uc.get_panel(&panel_id).await?;
    Ok(single_dto(detail))
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
    Ok(map_to_dtos(panels))
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
    rbac: Option<Extension<RoleContext>>,
    Path(panel_id): Path<String>,
) -> Result<Json<()>, ApiError> {
    // Gate RBAC : admin+ pour supprimer un panel. On passe par le use case
    // pour recuperer le guild_id (plus de SQL direct dans le handler).
    if rbac.is_some() {
        if let Ok(detail) = state.role_panels_uc.get_panel(&panel_id).await {
            check_role_for_guild(
                &state,
                &rbac,
                &detail.panel.guild_id,
                Role::Admin,
                "admin+ requis pour supprimer un panel",
            )
            .await?;
        }
    }
    state.role_panels_uc.delete_panel(&panel_id).await?;
    Ok(Json(()))
}

pub async fn list_auto_roles(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<AutoRoleDto>>, ApiError> {
    let roles = state.role_panels_uc.list_auto_roles(&guild_id).await?;
    Ok(map_to_dtos(roles))
}

pub async fn add_auto_role(
    State(state): State<AppState>,
    Json(dto): Json<CreateAutoRoleDto>,
) -> Result<Json<AutoRoleDto>, ApiError> {
    let role = state.role_panels_uc.add_auto_role(dto.into()).await?;
    Ok(single_dto(role))
}

pub async fn delete_auto_role(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, role_id)): Path<(String, String)>,
) -> Result<Json<()>, ApiError> {
    // Phase 7 B — Gate RBAC : admin+ pour toucher aux auto-roles.
    if let Some(Extension(ctx)) = rbac {
        require_role(&ctx, Role::Admin)
            .map_err(|_| ApiError(DomainError::Forbidden("admin+ requis pour supprimer un auto-role".into())))?;
    }
    state.role_panels_uc.delete_auto_role(&guild_id, &role_id).await?;
    Ok(Json(()))
}
