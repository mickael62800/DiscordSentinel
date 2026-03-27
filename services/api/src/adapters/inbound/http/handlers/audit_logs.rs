use axum::extract::{Query, State};
use axum::Json;

use crate::adapters::inbound::http::dto::audit_logs::{
    AuditLogQueryParams, AuditLogResponseDto, CreateAuditLogDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::manage_audit_logs::AuditLogFilters;

pub async fn create_audit_log(
    State(state): State<AppState>,
    Json(dto): Json<CreateAuditLogDto>,
) -> Result<Json<AuditLogResponseDto>, ApiError> {
    let log = state.audit_logs_uc.create(dto.into()).await?;
    Ok(Json(AuditLogResponseDto::from(log)))
}

pub async fn list_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<AuditLogQueryParams>,
) -> Result<Json<Vec<AuditLogResponseDto>>, ApiError> {
    let filters = AuditLogFilters {
        event_type: params.event_type,
        actor_id: params.actor_id,
        target_id: params.target_id,
        limit: params.limit.unwrap_or(100).min(500),
        offset: params.offset.unwrap_or(0),
    };

    let logs = state
        .audit_logs_uc
        .list(params.guild_id.as_deref(), filters)
        .await?;
    let dtos: Vec<AuditLogResponseDto> = logs.into_iter().map(AuditLogResponseDto::from).collect();
    Ok(Json(dtos))
}
