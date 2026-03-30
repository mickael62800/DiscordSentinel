use axum::extract::{Query, State};
use axum::Json;

use crate::adapters::inbound::http::dto::security::{
    ReportEventDto, SecurityEventResponseDto, SecurityQueryParams,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, single_dto};
use crate::adapters::inbound::http::state::AppState;

/// POST /api/security/events — signaler un événement de sécurité (depuis le security-bot)
pub async fn report_event(
    State(state): State<AppState>,
    Json(dto): Json<ReportEventDto>,
) -> Result<Json<SecurityEventResponseDto>, ApiError> {
    let event_type = dto.event_type.clone();
    let severity = dto.severity.clone();
    let description = dto.description.clone();
    let guild_id = dto.guild_id.clone();

    let command = dto.into();
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
