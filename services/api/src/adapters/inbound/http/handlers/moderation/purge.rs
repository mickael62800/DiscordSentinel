use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use tracing::info;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::adapters::inbound::http::middleware::rbac::require_superadmin;
use crate::domain::enums::system::role::Role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Deserialize)]
pub struct PurgeByDaysDto {
    pub guild_id: GuildId,
    pub days: i32,
}

#[derive(Debug, Deserialize)]
pub struct PurgeLogsDto {
    pub days: i32,
}

/// DELETE /api/purge/infractions — purge infractions plus vieilles que X jours
/// pour une guild. `days = 0` signifie "tout supprimer" (pas de filtre de date).
pub async fn purge_infractions(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<PurgeByDaysDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    crate::domain::entities::moderation::purge::validate_purge_days_allow_zero(dto.days)
        .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?;

    // Phase 7 B — Gate RBAC : owner requis pour une purge massive.
    check_role_for_guild(
        &state,
        &rbac,
        &dto.guild_id,
        Role::Owner,
        "owner requis pour purger des infractions",
    )
    .await?;

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
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<PurgeByDaysDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    crate::domain::entities::moderation::purge::validate_purge_days_strictly_positive(dto.days)
        .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?;

    // Phase 7 B — Gate RBAC : owner requis pour purger l'audit log.
    check_role_for_guild(
        &state,
        &rbac,
        &dto.guild_id,
        Role::Owner,
        "owner requis pour purger les audit logs",
    )
    .await?;

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
///
/// **Phase 7 B — Gate superadmin** : cet endpoint est GLOBAL (purge les logs
/// de TOUTES les guilds), donc `require_role_for_guild` n'a aucun sens ici.
/// On utilise `require_superadmin` qui check contre la liste statique
/// `SUPERADMIN_USER_IDS` (env var). Les appels bot/internal (pas de
/// `X-Discord-Token`) restent en pass-through.
pub async fn purge_logs(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<PurgeLogsDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    crate::domain::entities::moderation::purge::validate_purge_days_strictly_positive(dto.days)
        .map_err(|m| ApiError(DomainError::ValidationError(m.into())))?;

    // Phase 7 B — Gate superadmin pour les appels desktop.
    if let Some(Extension(ctx)) = rbac {
        require_superadmin(&state, &ctx)
            .map_err(|_| ApiError(DomainError::Forbidden("superadmin requis pour purger les logs systeme".into())))?;
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

#[cfg(test)]
#[path = "tests/purge.rs"]
mod tests;
