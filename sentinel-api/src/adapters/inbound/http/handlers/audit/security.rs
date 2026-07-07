use crate::adapters::inbound::http::dto::audit::security::ReportEventDto;
use crate::adapters::inbound::http::dto::audit::security::SecurityEventResponseDto;
use crate::adapters::inbound::http::dto::audit::security::SecurityQueryParams;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::rbac::{
    check_role, check_role_for_guild, RoleContext,
};
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::enums::system::role::Role;
use crate::adapters::inbound::http::validation;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;

/// POST /api/security/events — signaler un événement de sécurité (depuis le security-bot)
pub async fn report_event(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<ReportEventDto>,
) -> Result<Json<SecurityEventResponseDto>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_reason(&dto.description).map_err(ApiError)?;
    // Reserve au security-bot (Bearer API_KEY -> Internal, bypass) et aux admins
    // du serveur concerne. Empeche un user web de forger des evenements de
    // securite (faux positifs / watched users) pour une guilde arbitraire.
    check_role_for_guild(
        &state,
        &rbac,
        &dto.guild_id,
        Role::Admin,
        "role insuffisant pour signaler un evenement de securite",
    )
    .await?;

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
        &state,
        &rbac,
        &guild_id,
        "db.purge.security_events",
        "role insuffisant pour purger les evenements de securite",
    )
    .await?;

    // La purge (SQL) vit dans le use case / repo (plus de SQL inline).
    let (deleted_events, deleted_watches) = state.security_uc.purge_events(&guild_id).await?;

    Ok(Json(serde_json::json!({
        "deleted_events": deleted_events,
        "deleted_watches": deleted_watches,
    })))
}

/// GET /api/security/events — lister les événements de sécurité
pub async fn list_events(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(params): Query<SecurityQueryParams>,
) -> Result<Json<Vec<SecurityEventResponseDto>>, ApiError> {
    // Scope : un appelant web doit etre modo du serveur demande ; la liste
    // globale (sans guild_id) est reservee aux appels internes/superadmin.
    match params.guild_id.as_deref() {
        Some(gid) => {
            check_role_for_guild(&state, &rbac, gid, Role::Moderator, "role insuffisant").await?
        }
        None => check_role(
            &rbac,
            Role::Owner,
            "guild_id requis pour lister les evenements de securite",
        )?,
    }
    let events = state
        .security_uc
        .list_events(params.guild_id.as_deref())
        .await?;

    Ok(map_to_dtos(events))
}
