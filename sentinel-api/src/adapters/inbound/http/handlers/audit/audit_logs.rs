use crate::adapters::inbound::http::dto::audit::audit_logs::AuditLogQueryParams;
use crate::adapters::inbound::http::dto::audit::audit_logs::AuditLogResponseDto;
use crate::adapters::inbound::http::dto::audit::audit_logs::CreateAuditLogDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::normalize_limit;
use crate::adapters::inbound::http::helpers::normalize_offset;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::audit::manage_audit_logs::AuditLogFilters;
use sentinel_core::domain::enums::system::role::Role;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use sentinel_core::domain::errors::DomainError;

pub async fn create_audit_log(
    State(state): State<AppState>,
    Json(dto): Json<CreateAuditLogDto>,
) -> Result<Json<AuditLogResponseDto>, ApiError> {
    let log = state.audit_logs_uc.create(dto.into()).await?;
    Ok(single_dto(log))
}

/// DELETE /api/audit-logs/{guild_id} — purge les audit logs d'une guild
/// anterieurs a 0 jours (= tout). Passe par le use case, pas de SQL direct.
pub async fn purge_audit_logs(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<serde_json::Value>, ApiError> {
    crate::adapters::inbound::http::middleware::component_gates::check_component_role(
        &state,
        &rbac,
        &guild_id,
        "db.purge.audit_logs",
        "role insuffisant pour purger les audit logs",
    )
    .await?;

    let deleted = state
        .audit_logs_uc
        .delete_older_than_days(&guild_id, 0)
        .await?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

pub async fn list_audit_logs(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(params): Query<AuditLogQueryParams>,
) -> Result<Json<Vec<AuditLogResponseDto>>, ApiError> {
    // Securite : guild_id obligatoire pour eviter une fuite inter-guild.
    let guild_id = params.guild_id.ok_or_else(|| {
        ApiError(DomainError::ValidationError(
            "guild_id est obligatoire".into(),
        ))
    })?;

    // IDOR : le gate global ne protege JAMAIS les GET -> sans cette garde, tout
    // appelant lisait les audit logs (qui a banni/mute qui) de n'importe quel
    // serveur en changeant guild_id. Reserve moderator+ scope guilde.
    check_role_for_guild(
        &state,
        &rbac,
        &guild_id,
        Role::Moderator,
        "moderator+ requis pour lire les audit logs",
    )
    .await?;

    let filters = AuditLogFilters {
        event_type: params.event_type,
        actor_id: params.actor_id,
        target_id: params.target_id,
        limit: normalize_limit(params.limit, 100, 500),
        offset: normalize_offset(params.offset),
    };

    let logs = state.audit_logs_uc.list(Some(&guild_id), filters).await?;
    Ok(map_to_dtos(logs))
}
