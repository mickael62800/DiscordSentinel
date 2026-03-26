use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::adapters::inbound::http::dto::rules::{CreateRuleDto, RuleResponseDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

pub async fn get_rules(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<RuleResponseDto>>, ApiError> {
    let rules = state.rules_uc.get_rules(&guild_id).await?;
    let dtos: Vec<RuleResponseDto> = rules.into_iter().map(RuleResponseDto::from).collect();
    Ok(Json(dtos))
}

pub async fn create_rule(
    State(state): State<AppState>,
    Json(dto): Json<CreateRuleDto>,
) -> Result<Json<RuleResponseDto>, ApiError> {
    let command = dto.into();
    let rule = state.rules_uc.create_or_update_rule(command).await?;
    Ok(Json(RuleResponseDto::from(rule)))
}

pub async fn delete_rule(
    State(state): State<AppState>,
    Path((guild_id, rule_id)): Path<(String, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.rules_uc.delete_rule(&guild_id, rule_id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
