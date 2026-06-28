use axum::extract::Path;
use axum::extract::State;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::Extension;
use axum::Json;
use uuid::Uuid;

use crate::adapters::inbound::http::dto::moderation::rules::CreateRuleDto;
use crate::adapters::inbound::http::dto::moderation::rules::RuleResponseDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::rbac::check_role;
use sentinel_core::domain::enums::system::role::Role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;

pub async fn get_rules(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<RuleResponseDto>>, ApiError> {
    let rules = state.rules_uc.get_rules(&guild_id).await?;
    Ok(map_to_dtos(rules))
}

pub async fn create_rule(
    State(state): State<AppState>,
    Json(dto): Json<CreateRuleDto>,
) -> Result<Json<RuleResponseDto>, ApiError> {
    let command = dto.into();
    let rule = state.rules_uc.create_or_update_rule(command).await?;
    Ok(single_dto(rule))
}

pub async fn delete_rule(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, rule_id)): Path<(String, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_role(&rbac, Role::Admin, "admin+ requis pour supprimer une regle")?;
    state.rules_uc.delete_rule(&guild_id, rule_id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
