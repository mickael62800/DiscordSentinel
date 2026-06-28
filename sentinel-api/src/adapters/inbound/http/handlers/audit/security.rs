use axum::extract::Query;
use axum::extract::State;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::Extension;
use axum::Json;
use crate::adapters::inbound::http::dto::audit::security::ReportEventDto;
use crate::adapters::inbound::http::dto::audit::security::SecurityEventResponseDto;
use crate::adapters::inbound::http::dto::audit::security::SecurityQueryParams;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::errors_helpers::sqlx_internal;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;

/// POST /api/security/events — signaler un événement de sécurité (depuis le security-bot)
pub async fn report_event(
    State(state): State<AppState>,
    Json(dto): Json<ReportEventDto>,
) -> Result<Json<SecurityEventResponseDto>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_reason(&dto.description).map_err(ApiError)?;

    let (command, (event_type, severity, description, guild_id)) =
        crate::capture_and_into!(dto, event_type, severity, description, guild_id);
    let event = state.security_uc.report_event(command).await?;

    // Broadcast WebSocket pour l'app desktop
    state.broadcaster.broadcast(
        "security_event",
        serde_json::json!({
            "guild_id": guild_id,
            "event_type": event_type,
            "severity": severity,
            "description": description,
        }),
    );

    Ok(single_dto(event))
}

/// DELETE /api/security/events/{guild_id}
/// Purge tous les evenements de securite d'une guild + les manual_watched_users
/// crees automatiquement par ces evenements.
pub async fn purge_events(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<serde_json::Value>, ApiError> {
    crate::adapters::inbound::http::middleware::component_gates::check_component_role(
        &state, &rbac, &guild_id, "db.purge.security_events",
        "role insuffisant pour purger les evenements de securite",
    )
    .await?;

    // Phase 4 : on supprime depuis audit_logs (la table security_events est
    // deprecated, plus de writes). On vire aussi les anciennes lignes legacy.
    let events_audit = sqlx::query(
        "DELETE FROM audit_logs WHERE guild_id = $1 AND event_type LIKE 'security_%'",
    )
    .bind(&guild_id)
    .execute(&state.pg_pool)
    .await
    .map_err(sqlx_internal("purge audit security"))?;

    let _ = sqlx::query("DELETE FROM security_events WHERE guild_id = $1")
        .bind(&guild_id)
        .execute(&state.pg_pool)
        .await;

    let events = events_audit;

    let watched = sqlx::query(
        "DELETE FROM manual_watched_users WHERE guild_id = $1 AND added_by = 'security_event'",
    )
    .bind(&guild_id)
    .execute(&state.pg_pool)
    .await
    .map_err(sqlx_internal("purge auto watch"))?;

    Ok(Json(serde_json::json!({
        "deleted_events": events.rows_affected(),
        "deleted_watches": watched.rows_affected(),
    })))
}

/// GET /api/security/events — lister les événements de sécurité
pub async fn list_events(
    State(state): State<AppState>,
    Query(params): Query<SecurityQueryParams>,
) -> Result<Json<Vec<SecurityEventResponseDto>>, ApiError> {
    let events = state
        .security_uc
        .list_events(params.guild_id.as_deref())
        .await?;

    Ok(map_to_dtos(events))
}
