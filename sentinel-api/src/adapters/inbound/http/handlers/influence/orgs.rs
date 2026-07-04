//! Handlers organisations (creation, info, adhesion, membres).

use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use sentinel_core::domain::enums::influence::organization_kind::OrganizationKind;
use sentinel_core::domain::errors::DomainError;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::handlers::influence::dto::{
    OrgInfoDto, OrgMemberDto, OrganizationDto,
};
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateOrgDto {
    pub founder_user_id: String,
    #[serde(default)]
    pub founder_username: String,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub motto: String,
}

#[derive(Debug, Deserialize)]
pub struct OrgNameDto {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct JoinOrgDto {
    pub name: String,
    pub user_id: String,
    #[serde(default)]
    pub username: String,
}

/// POST /api/influence/{guild_id}/orgs
pub async fn create_org(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<CreateOrgDto>,
) -> Result<Json<OrganizationDto>, ApiError> {
    let kind = OrganizationKind::from_str_lossy(&dto.kind).ok_or_else(|| {
        ApiError(DomainError::ValidationError(format!(
            "Type d'organisation invalide : {}",
            dto.kind
        )))
    })?;
    let org = state
        .influence_orgs_uc
        .create(
            &guild_id,
            &dto.founder_user_id,
            &dto.founder_username,
            kind,
            &dto.name,
            &dto.motto,
        )
        .await?;
    Ok(Json(org.into()))
}

/// POST /api/influence/{guild_id}/orgs/info
pub async fn org_info(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<OrgNameDto>,
) -> Result<Json<OrgInfoDto>, ApiError> {
    let info = state.influence_orgs_uc.info(&guild_id, &dto.name).await?;
    Ok(Json(info.into()))
}

/// POST /api/influence/{guild_id}/orgs/join
pub async fn join_org(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<JoinOrgDto>,
) -> Result<Json<OrganizationDto>, ApiError> {
    let org = state
        .influence_orgs_uc
        .join(&guild_id, &dto.name, &dto.user_id, &dto.username)
        .await?;
    Ok(Json(org.into()))
}

/// POST /api/influence/{guild_id}/orgs/members
pub async fn org_members(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<OrgNameDto>,
) -> Result<Json<Vec<OrgMemberDto>>, ApiError> {
    let members = state.influence_orgs_uc.members(&guild_id, &dto.name).await?;
    Ok(Json(members.into_iter().map(Into::into).collect()))
}
