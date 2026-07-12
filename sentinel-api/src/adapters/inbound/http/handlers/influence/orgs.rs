//! Handlers organisations (creation, info, adhesion, membres).

use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use sentinel_core::domain::entities::influence::archive::RelationKind;
use sentinel_core::domain::enums::influence::organization_kind::OrganizationKind;
use sentinel_core::domain::errors::DomainError;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::handlers::influence::dto::{
    DividendResultDto, FundLawDto, FundingResultDto, OrgInfoDto, OrgMemberDto, OrgRankDto,
    OrganizationDto, PayMemberDto, TreasuryOpDto, TreasuryViewDto,
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

#[derive(Debug, Deserialize)]
pub struct DissolveOrgDto {
    pub name: String,
    pub actor_user_id: String,
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

/// POST /api/influence/{guild_id}/orgs/dissolve — dissout une org (fondateur).
/// Renvoie l'org dissoute (avec discord_channel_id) pour que le bot supprime
/// les salons associes.
pub async fn dissolve_org(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<DissolveOrgDto>,
) -> Result<Json<OrganizationDto>, ApiError> {
    let org = state
        .influence_orgs_uc
        .dissolve(&guild_id, &dto.name, &dto.actor_user_id)
        .await?;
    Ok(Json(org.into()))
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

/// POST /api/influence/{guild_id}/orgs/ranking — palmares par tresor de guerre.
pub async fn org_ranking(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<OrgRankDto>>, ApiError> {
    let entries = state.influence_orgs_uc.ranking(&guild_id).await?;
    Ok(Json(entries.into_iter().map(Into::into).collect()))
}

/// POST /api/influence/{guild_id}/orgs/treasury — consulte la tresorerie.
pub async fn org_treasury(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<OrgNameDto>,
) -> Result<Json<TreasuryViewDto>, ApiError> {
    let v = state.influence_orgs_uc.treasury(&guild_id, &dto.name).await?;
    Ok(Json(v.into()))
}

/// POST /api/influence/{guild_id}/orgs/treasury/deposit — reverse des coins.
pub async fn treasury_deposit(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<TreasuryOpDto>,
) -> Result<Json<TreasuryViewDto>, ApiError> {
    let v = state
        .influence_orgs_uc
        .deposit_treasury(
            &guild_id,
            &dto.name,
            &dto.actor_user_id,
            &dto.actor_username,
            dto.amount,
        )
        .await?;
    Ok(Json(v.into()))
}

/// POST /api/influence/{guild_id}/orgs/treasury/withdraw — retire des coins.
pub async fn treasury_withdraw(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<TreasuryOpDto>,
) -> Result<Json<TreasuryViewDto>, ApiError> {
    let v = state
        .influence_orgs_uc
        .withdraw_treasury(
            &guild_id,
            &dto.name,
            &dto.actor_user_id,
            &dto.actor_username,
            dto.amount,
        )
        .await?;
    Ok(Json(v.into()))
}

/// POST /api/influence/{guild_id}/orgs/fund-law — finance une loi (lobbying).
pub async fn fund_law(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<FundLawDto>,
) -> Result<Json<FundingResultDto>, ApiError> {
    let r = state
        .influence_orgs_uc
        .fund_law(
            &guild_id,
            &dto.name,
            &dto.law_id,
            &dto.actor_user_id,
            &dto.actor_username,
            dto.amount,
            dto.camp_pour,
        )
        .await?;
    Ok(Json(r.into()))
}

/// POST /api/influence/{guild_id}/orgs/treasury/dividend — verse a chaque membre.
pub async fn treasury_dividend(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<TreasuryOpDto>,
) -> Result<Json<DividendResultDto>, ApiError> {
    let r = state
        .influence_orgs_uc
        .distribute_dividend(
            &guild_id,
            &dto.name,
            &dto.actor_user_id,
            &dto.actor_username,
            dto.amount,
        )
        .await?;
    Ok(Json(r.into()))
}

/// POST /api/influence/{guild_id}/orgs/treasury/pay — paie un membre.
pub async fn treasury_pay(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<PayMemberDto>,
) -> Result<Json<TreasuryViewDto>, ApiError> {
    let v = state
        .influence_orgs_uc
        .pay_member(
            &guild_id,
            &dto.name,
            &dto.actor_user_id,
            &dto.actor_username,
            &dto.beneficiary_user_id,
            &dto.beneficiary_username,
            dto.amount,
        )
        .await?;
    Ok(Json(v.into()))
}

#[derive(Debug, Deserialize)]
pub struct PrepareRoleDto {
    pub actor_user_id: String,
    #[serde(default)]
    pub actor_username: String,
    #[serde(default)]
    pub is_moderator: bool,
    pub org_name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct RolePrepDto {
    pub founder_user_id: String,
    pub org_name: String,
}

/// POST /api/influence/{guild_id}/orgs/role/prepare
pub async fn prepare_role(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<PrepareRoleDto>,
) -> Result<Json<RolePrepDto>, ApiError> {
    let prep = state
        .influence_orgs_uc
        .prepare_role(
            &guild_id,
            &dto.actor_user_id,
            &dto.actor_username,
            dto.is_moderator,
            &dto.org_name,
        )
        .await?;
    Ok(Json(RolePrepDto {
        founder_user_id: prep.founder_user_id,
        org_name: prep.org_name,
    }))
}

#[derive(Debug, Deserialize)]
pub struct LinkRoleDto {
    pub org_name: String,
    pub role_id: String,
    #[serde(default)]
    pub actor_user_id: String,
    #[serde(default)]
    pub is_moderator: bool,
}

/// POST /api/influence/{guild_id}/orgs/role/link
pub async fn link_role(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<LinkRoleDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .influence_orgs_uc
        .set_role(
            &guild_id,
            &dto.org_name,
            &dto.role_id,
            &dto.actor_user_id,
            dto.is_moderator,
        )
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct LinkChannelDto {
    pub org_name: String,
    pub channel_id: String,
}

/// POST /api/influence/{guild_id}/orgs/channel/link
pub async fn link_channel(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<LinkChannelDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .influence_orgs_uc
        .set_channel(&guild_id, &dto.org_name, &dto.channel_id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct SetRelationDto {
    pub actor_user_id: String,
    #[serde(default)]
    pub actor_username: String,
    pub org_name: String,
    pub other_org_name: String,
    pub relation: String,
}

/// POST /api/influence/{guild_id}/orgs/relation
pub async fn set_relation(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<SetRelationDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let relation = RelationKind::from_str_lossy(&dto.relation).ok_or_else(|| {
        ApiError(DomainError::ValidationError(format!(
            "Relation invalide : {}",
            dto.relation
        )))
    })?;
    state
        .influence_orgs_uc
        .set_relation(
            &guild_id,
            &dto.actor_user_id,
            &dto.actor_username,
            &dto.org_name,
            &dto.other_org_name,
            relation,
        )
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
