use axum::extract::{Path, State};
use axum::{Extension, Json};
use uuid::Uuid;

use crate::adapters::inbound::http::dto::rules::{CreateRuleDto, RuleResponseDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, single_dto};
use crate::adapters::inbound::http::middleware::rbac::{require_role, Role, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

pub async fn get_rules(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
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
    // Phase 7 B — Gate RBAC : admin+ requis pour modifier les regles AutoMod.
    // L'Option<Extension> preserve le pass-through pour les appels bot/internal
    // qui n'ont pas de X-Discord-Token (le middleware rbac ne les injecte pas).
    if let Some(Extension(ctx)) = rbac {
        require_role(&ctx, Role::Admin)
            .map_err(|_| ApiError(DomainError::Forbidden("admin+ requis pour supprimer une regle".into())))?;
    }
    state.rules_uc.delete_rule(&guild_id, rule_id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
