use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use tracing::info;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;

#[derive(Debug, Deserialize)]
pub struct PurgeByDaysDto {
    pub guild_id: String,
    pub days: i32,
}

#[derive(Debug, Deserialize)]
pub struct PurgeLogsDto {
    pub days: i32,
}

/// DELETE /api/purge/infractions — purge infractions older than X days for a guild
pub async fn purge_infractions(
    State(state): State<AppState>,
    Json(dto): Json<PurgeByDaysDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    if dto.days < 1 {
        return Err(ApiError(crate::domain::errors::DomainError::ValidationError("days doit etre >= 1".into())));
    }

    let count = state.infractions_uc.delete_older_than_days(&dto.guild_id, dto.days).await?;
    info!(guild_id = %dto.guild_id, days = dto.days, deleted = count, "Purge infractions");

    state.broadcaster.broadcast("purge_completed", serde_json::json!({
        "type": "infractions",
        "guild_id": &dto.guild_id,
        "days": dto.days,
        "deleted": count,
    }));

    Ok(Json(serde_json::json!({ "deleted": count })))
}

/// DELETE /api/purge/audit-logs — purge audit logs older than X days for a guild
pub async fn purge_audit_logs(
    State(state): State<AppState>,
    Json(dto): Json<PurgeByDaysDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    if dto.days < 1 {
        return Err(ApiError(crate::domain::errors::DomainError::ValidationError("days doit etre >= 1".into())));
    }

    let count = state.audit_logs_uc.delete_older_than_days(&dto.guild_id, dto.days).await?;
    info!(guild_id = %dto.guild_id, days = dto.days, deleted = count, "Purge audit logs");

    state.broadcaster.broadcast("purge_completed", serde_json::json!({
        "type": "audit_logs",
        "guild_id": &dto.guild_id,
        "days": dto.days,
        "deleted": count,
    }));

    Ok(Json(serde_json::json!({ "deleted": count })))
}

/// DELETE /api/purge/logs — purge system logs older than X days (global, not guild-scoped)
pub async fn purge_logs(
    State(state): State<AppState>,
    Json(dto): Json<PurgeLogsDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if dto.days < 1 {
        return Err(ApiError(crate::domain::errors::DomainError::ValidationError("days doit etre >= 1".into())));
    }

    let count = state.log_repo.delete_older_than_days(dto.days).await?;
    info!(days = dto.days, deleted = count, "Purge logs systeme");

    state.broadcaster.broadcast("purge_completed", serde_json::json!({
        "type": "logs",
        "days": dto.days,
        "deleted": count,
    }));

    Ok(Json(serde_json::json!({ "deleted": count })))
}
